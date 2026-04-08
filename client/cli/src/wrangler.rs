use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use tempfile::tempdir;

const WORKER_NAME: &str = "rift-test-proxy";
const COMPATIBILITY_DATE: &str = "2026-04-07";
const RIFT_WORKER_SCRIPT: &str = r#"export default {
  async fetch(request) {
    const url = new URL(request.url);
    const host = url.hostname;
    const origin = "https://api.riftl.ink";
    const upstream = new URL(url.pathname + url.search, origin);
    const headers = new Headers(request.headers);
    headers.set("X-Rift-Host", host);
    const response = await fetch(upstream.toString(), {
      method: request.method,
      headers,
      body: request.method !== "GET" && request.method !== "HEAD"
        ? request.body
        : undefined,
      redirect: "manual",
    });
    return response;
  },
};"#;

#[derive(Debug)]
pub enum WranglerError {
    MissingNode,
    InstallFailed(String),
    LoginFailed(String),
    NotAuthenticated,
    ExistingWorkerConflict,
    CommandFailed(String),
}

impl std::fmt::Display for WranglerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingNode => write!(f, "Wrangler requires Node/npm, but npm is not installed."),
            Self::InstallFailed(message) => write!(f, "Failed to install Wrangler: {message}"),
            Self::LoginFailed(message) => write!(f, "Cloudflare login failed: {message}"),
            Self::NotAuthenticated => write!(
                f,
                "Wrangler is installed but not authenticated with Cloudflare."
            ),
            Self::ExistingWorkerConflict => write!(
                f,
                "A worker named `rift-proxy` already exists and needs confirmation before Rift updates it."
            ),
            Self::CommandFailed(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for WranglerError {}

#[derive(Debug, Clone)]
pub enum WranglerInvocation {
    Path(PathBuf),
}

#[derive(Debug, Clone, Deserialize)]
struct WhoAmI {
    #[serde(default)]
    accounts: Vec<CloudflareAccount>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CloudflareAccount {
    pub id: String,
    pub name: String,
}

pub fn detect() -> Option<WranglerInvocation> {
    which("wrangler").map(WranglerInvocation::Path)
}

pub fn install() -> Result<WranglerInvocation, WranglerError> {
    let Some(npm) = which("npm") else {
        return Err(WranglerError::MissingNode);
    };

    let status = Command::new(&npm)
        .args(["install", "-g", "wrangler"])
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|e| WranglerError::InstallFailed(e.to_string()))?;

    if !status.success() {
        return Err(WranglerError::InstallFailed(
            "npm install returned a non-zero exit status".to_string(),
        ));
    }

    detect()
        .or_else(|| locate_from_npm_prefix(&npm))
        .ok_or_else(|| {
            WranglerError::InstallFailed(
                "Wrangler was installed, but Rift could not find the executable.".to_string(),
            )
        })
}

pub fn ensure_login(
    wrangler: &WranglerInvocation,
) -> Result<Vec<CloudflareAccount>, WranglerError> {
    match whoami(wrangler) {
        Ok(accounts) => Ok(accounts),
        Err(WranglerError::NotAuthenticated) => {
            let status = run_interactive(wrangler, &["login"])?;
            if !status.success() {
                return Err(WranglerError::LoginFailed(
                    "Wrangler login did not complete successfully.".to_string(),
                ));
            }
            whoami(wrangler)
        }
        Err(error) => Err(error),
    }
}

pub fn detect_existing_worker(
    wrangler: &WranglerInvocation,
    account_id: Option<&str>,
) -> Result<bool, WranglerError> {
    let temp = tempdir().map_err(|e| WranglerError::CommandFailed(e.to_string()))?;
    let config_path = temp.path().join("wrangler.jsonc");
    fs::write(
        &config_path,
        format!(
            r#"{{
  "name": "{WORKER_NAME}",
  "main": "worker.js",
  "compatibility_date": "{COMPATIBILITY_DATE}"
}}"#
        ),
    )
    .map_err(|e| WranglerError::CommandFailed(e.to_string()))?;
    fs::write(temp.path().join("worker.js"), RIFT_WORKER_SCRIPT)
        .map_err(|e| WranglerError::CommandFailed(e.to_string()))?;

    let output = run_capture(
        wrangler,
        &[
            "deployments",
            "list",
            "--config",
            config_path.to_string_lossy().as_ref(),
        ],
        account_id,
    )?;

    if output.status.success() {
        return Ok(true);
    }

    let combined = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    if combined.contains("not authenticated") || combined.contains("login") {
        return Err(WranglerError::NotAuthenticated);
    }
    if combined.contains("No Worker named")
        || combined.contains("There are currently no deployments")
        || combined.contains("Could not find")
    {
        return Ok(false);
    }

    Ok(false)
}

pub fn deploy_worker(
    wrangler: &WranglerInvocation,
    domain: &str,
    root_zone: &str,
    account_id: Option<&str>,
) -> Result<(), WranglerError> {
    let temp = tempdir().map_err(|e| WranglerError::CommandFailed(e.to_string()))?;
    let worker_path = temp.path().join("worker.js");
    let config_path = temp.path().join("wrangler.jsonc");
    fs::write(&worker_path, RIFT_WORKER_SCRIPT)
        .map_err(|e| WranglerError::CommandFailed(e.to_string()))?;
    fs::write(
        &config_path,
        format!(
            r#"{{
  "name": "{WORKER_NAME}",
  "main": "{}",
  "compatibility_date": "{COMPATIBILITY_DATE}",
  "routes": [
    {{
      "pattern": "{domain}",
      "custom_domain": true
    }},
    {{
      "pattern": "{domain}/*",
      "zone_name": "{root_zone}"
    }}
  ]
}}"#,
            worker_path.to_string_lossy()
        ),
    )
    .map_err(|e| WranglerError::CommandFailed(e.to_string()))?;

    let output = run_capture(
        wrangler,
        &["deploy", "--config", config_path.to_string_lossy().as_ref()],
        account_id,
    )?;

    print!("{}", String::from_utf8_lossy(&output.stdout));
    eprint!("{}", String::from_utf8_lossy(&output.stderr));

    if output.status.success() {
        Ok(())
    } else {
        let combined = format!(
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        if combined.contains("Could not find zone for") {
            return Err(WranglerError::CommandFailed(format!(
                "Wrangler could not find the Cloudflare zone for `{domain}`. This usually means Wrangler is logged into the wrong Cloudflare account, or `{root_zone}` is not managed in the current account. Run `wrangler whoami`, then `wrangler logout` and `wrangler login` if needed."
            )));
        }
        Err(WranglerError::CommandFailed(
            "Wrangler failed to deploy or attach the Rift worker.".to_string(),
        ))
    }
}

fn whoami(wrangler: &WranglerInvocation) -> Result<Vec<CloudflareAccount>, WranglerError> {
    let output = run_capture(wrangler, &["whoami", "--json"], None)?;
    if !output.status.success() {
        return Err(WranglerError::NotAuthenticated);
    }

    let info: WhoAmI = serde_json::from_slice(&output.stdout).map_err(|e| {
        WranglerError::CommandFailed(format!("Failed to parse Wrangler whoami output: {e}"))
    })?;
    Ok(info.accounts.into_iter().collect())
}

fn run_capture(
    wrangler: &WranglerInvocation,
    args: &[&str],
    account_id: Option<&str>,
) -> Result<Output, WranglerError> {
    let mut command = base_command(wrangler, account_id);
    command.args(args);
    command
        .output()
        .map_err(|e| WranglerError::CommandFailed(e.to_string()))
}

fn run_interactive(
    wrangler: &WranglerInvocation,
    args: &[&str],
) -> Result<std::process::ExitStatus, WranglerError> {
    let mut command = base_command(wrangler, None);
    command.args(args);
    command
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|e| WranglerError::CommandFailed(e.to_string()))
}

fn base_command(wrangler: &WranglerInvocation, account_id: Option<&str>) -> Command {
    let mut command = match wrangler {
        WranglerInvocation::Path(path) => Command::new(path),
    };
    if let Some(account_id) = account_id {
        command.env("CLOUDFLARE_ACCOUNT_ID", account_id);
    }
    command
}

fn locate_from_npm_prefix(npm: &Path) -> Option<WranglerInvocation> {
    let output = Command::new(npm).args(["prefix", "-g"]).output().ok()?;
    if !output.status.success() {
        return None;
    }

    let prefix = String::from_utf8(output.stdout).ok()?;
    let candidate = Path::new(prefix.trim()).join("bin").join("wrangler");
    candidate
        .exists()
        .then_some(WranglerInvocation::Path(candidate))
}

fn which(binary: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(binary))
        .find(|candidate| candidate.exists())
        .or_else(|| {
            if cfg!(windows) {
                std::env::split_paths(&path)
                    .map(|dir| dir.join(format!("{binary}.cmd")))
                    .find(|candidate| candidate.exists())
            } else {
                None
            }
        })
}
