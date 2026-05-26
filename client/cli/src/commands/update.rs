//! `rift update` — fetch the latest CLI release from GitHub and atomically
//! replace the current binary.
//!
//! Mirrors the source-of-truth in `client/cli/install.sh` so a user who
//! installed via the curl-pipe-sh script and a user who runs `rift update`
//! end up with bit-identical binaries from the same GitHub release tarball.
//!
//! Windows is intentionally out of scope for now — replacing a running .exe
//! requires a different dance than POSIX `rename(2)`-over-open-file. Windows
//! users get a clear "rerun the installer" hint.

use std::io::Cursor;
use std::path::PathBuf;

use serde::Deserialize;

use crate::commands::version::host_target;
use crate::error::CliError;
use crate::ui;

const REPO: &str = "saltyskip/rift";
const TAG_PREFIX: &str = "rift-cli-v";

pub struct Args {
    /// Don't install — just check whether a newer version exists.
    pub check: bool,
    /// Pin to a specific version (e.g. `0.1.3`). Without this the latest
    /// `rift-cli-v*` release wins.
    pub version: Option<String>,
    pub json: bool,
}

pub async fn run(args: Args) -> Result<(), CliError> {
    let current = env!("CARGO_PKG_VERSION");
    let target = host_target();

    if target.starts_with("unknown-") {
        return Err(CliError::General(format!(
            "Self-update isn't supported on this platform ({target}). Re-run the installer manually."
        )));
    }
    if std::env::consts::OS == "windows" {
        return Err(CliError::General(
            "Self-update on Windows isn't wired up yet — re-run the install command from the README to upgrade.".into(),
        ));
    }

    let release = match args.version.as_deref() {
        Some(v) => fetch_release_by_version(v).await?,
        None => fetch_latest_cli_release().await?,
    };
    let target_version = release.tag_name.trim_start_matches(TAG_PREFIX).to_string();

    // Compare semver-y (X.Y.Z, ignoring pre-release suffixes). `==` alone
    // misses two cases worth distinguishing:
    //   - local > latest: a dev build ahead of the last published release.
    //     Telling the user to "update" would downgrade them silently. Treat
    //     as up_to_date with a note unless they pinned `--version`.
    //   - local < latest: an actual upgrade. The remaining flow handles it.
    let ordering = args
        .version
        .is_none()
        .then(|| compare_semver(current, &target_version))
        .flatten();
    let already_current = matches!(
        ordering,
        Some(std::cmp::Ordering::Equal | std::cmp::Ordering::Greater)
    );

    if already_current {
        if args.json {
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "updated": false,
                    "current": current,
                    "latest": target_version,
                    "reason": "up_to_date",
                }))?
            );
        } else if matches!(ordering, Some(std::cmp::Ordering::Greater)) {
            ui::note(&format!(
                "Already on rift {current} (latest published release is {target_version})."
            ));
        } else {
            ui::note(&format!("Already up to date (rift {current})."));
        }
        return Ok(());
    }

    if args.check {
        if args.json {
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "updated": false,
                    "current": current,
                    "latest": target_version,
                    "reason": "check_only",
                }))?
            );
        } else {
            ui::heading(
                "Update available",
                &format!("rift {current} → {target_version}"),
            );
            ui::spacer();
            ui::note("Run `rift update` to install.");
        }
        return Ok(());
    }

    let archive_name = format!("rift-{target_version}-{target}.tar.gz");
    let asset_url = format!(
        "https://github.com/{REPO}/releases/download/{}/{archive_name}",
        release.tag_name
    );

    let spinner = ui::spinner(format!("Downloading rift {target_version}…"));
    let resp = reqwest::get(&asset_url)
        .await
        .map_err(|e| CliError::General(format!("Download failed for {asset_url}: {e}")))?;
    if !resp.status().is_success() {
        spinner.finish_and_clear();
        return Err(CliError::General(format!(
            "Download returned HTTP {} for {asset_url}",
            resp.status()
        )));
    }
    let bytes = resp
        .bytes()
        .await
        .map_err(|e| CliError::General(format!("Reading download body: {e}")))?;
    spinner.finish_and_clear();

    let new_binary = extract_binary(&bytes, &target_version, &target)?;

    // Atomic in-place replacement. `self_replace` handles the POSIX
    // rename-over-open-file dance, plus same-filesystem checks. The running
    // process keeps executing the old binary via its open file descriptor;
    // the next `rift` invocation runs the new one.
    self_replace::self_replace(&new_binary).map_err(|e| {
        CliError::General(format!(
            "Replacing the current binary failed: {e}. If `rift` lives in a root-owned directory (e.g. /usr/local/bin), retry with sudo."
        ))
    })?;

    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "updated": true,
                "current": current,
                "latest": target_version,
            }))?
        );
    } else {
        ui::success(&format!("Updated rift {current} → {target_version}"));
    }

    Ok(())
}

// ── Helpers ──

#[derive(Debug, Deserialize)]
struct GhRelease {
    tag_name: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
}

/// Latest published `rift-cli-v*` release. We list-and-filter rather than
/// hitting `/releases/latest` because this repo also publishes mobile SDK
/// releases under `sdk-v*` tags, and "latest" can land on those.
async fn fetch_latest_cli_release() -> Result<GhRelease, CliError> {
    let url = format!("https://api.github.com/repos/{REPO}/releases?per_page=30");
    let releases = github_get_releases(&url).await?;
    releases
        .into_iter()
        .find(|r| r.tag_name.starts_with(TAG_PREFIX) && !r.draft && !r.prerelease)
        .ok_or_else(|| CliError::General("No published CLI releases found.".into()))
}

async fn fetch_release_by_version(version: &str) -> Result<GhRelease, CliError> {
    let tag = format!("{TAG_PREFIX}{version}");
    let url = format!("https://api.github.com/repos/{REPO}/releases/tags/{tag}");
    let client = github_client()?;
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| CliError::General(format!("GitHub API request failed: {e}")))?;
    if resp.status() == reqwest::StatusCode::NOT_FOUND {
        return Err(CliError::General(format!(
            "No release tagged {tag}. Run `rift update --check` to see what's available."
        )));
    }
    if !resp.status().is_success() {
        return Err(CliError::General(format!(
            "GitHub API returned HTTP {} for {tag}",
            resp.status()
        )));
    }
    resp.json::<GhRelease>()
        .await
        .map_err(|e| CliError::General(format!("Parsing release JSON: {e}")))
}

async fn github_get_releases(url: &str) -> Result<Vec<GhRelease>, CliError> {
    let client = github_client()?;
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| CliError::General(format!("GitHub API request failed: {e}")))?;
    if !resp.status().is_success() {
        return Err(CliError::General(format!(
            "GitHub API returned HTTP {}",
            resp.status()
        )));
    }
    resp.json::<Vec<GhRelease>>()
        .await
        .map_err(|e| CliError::General(format!("Parsing releases JSON: {e}")))
}

/// Compare two `MAJOR.MINOR.PATCH` strings. Returns `None` if either side
/// has a non-numeric component or wrong arity — `update` then falls back to
/// "treat as not-equal" (proceeds to upgrade path), which is the safer
/// behavior than misreporting "up to date." We don't pull in the `semver`
/// crate because pre-release suffixes aren't part of how the CLI versions
/// are tagged today.
fn compare_semver(a: &str, b: &str) -> Option<std::cmp::Ordering> {
    let parse = |s: &str| -> Option<(u32, u32, u32)> {
        let mut parts = s.split('.');
        let major = parts.next()?.parse().ok()?;
        let minor = parts.next()?.parse().ok()?;
        let patch = parts.next()?.parse().ok()?;
        if parts.next().is_some() {
            return None;
        }
        Some((major, minor, patch))
    };
    Some(parse(a)?.cmp(&parse(b)?))
}

/// GitHub's API requires a `User-Agent` header on every request — without
/// it they 403 anonymous traffic. The version string lets them rate-limit /
/// track usage if it ever matters.
fn github_client() -> Result<reqwest::Client, CliError> {
    reqwest::Client::builder()
        .user_agent(format!("rift-cli/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| CliError::General(format!("HTTP client init: {e}")))
}

/// Untar the release archive into a tempdir, return the path to the
/// extracted `rift` binary. The install.sh layout is:
///
///   rift-{version}-{label}/
///     └─ rift              ← executable
///
/// Tempdir lifetime is bound to the process; `self_replace` is expected to
/// move-not-copy so we don't worry about cleanup races.
fn extract_binary(archive: &[u8], version: &str, target: &str) -> Result<PathBuf, CliError> {
    let tmp =
        tempfile::tempdir().map_err(|e| CliError::General(format!("creating tempdir: {e}")))?;
    let tmp_path = tmp.keep();

    let gz = flate2::read::GzDecoder::new(Cursor::new(archive));
    let mut tar = tar::Archive::new(gz);
    tar.unpack(&tmp_path)
        .map_err(|e| CliError::General(format!("unpacking archive: {e}")))?;

    let bin = tmp_path
        .join(format!("rift-{version}-{target}"))
        .join("rift");
    if !bin.exists() {
        return Err(CliError::General(format!(
            "Extracted archive missing expected layout at {}.",
            bin.display()
        )));
    }
    // self_replace needs the file to be executable on its way in.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&bin)
            .map_err(|e| CliError::General(format!("stat extracted binary: {e}")))?
            .permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&bin, perms)
            .map_err(|e| CliError::General(format!("chmod extracted binary: {e}")))?;
    }
    Ok(bin)
}
