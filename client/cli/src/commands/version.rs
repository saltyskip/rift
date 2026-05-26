//! `rift version` — print build version and platform target.
//!
//! `clap` already derives `--version` from the `#[command(version)]` attr on
//! the root `Cli` struct. This subcommand is the same value plus a bit more
//! context for debugging ("which binary is this, where did it come from,
//! what platform was it built for"). Mirrors what `gh version`, `vercel
//! --version` and similar tools do.

use crate::error::CliError;
use crate::ui;

pub fn run(json: bool) -> Result<(), CliError> {
    let version = env!("CARGO_PKG_VERSION");
    let target = host_target();
    let binary = std::env::current_exe()
        .ok()
        .map(|p| p.display().to_string());

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "version": version,
                "target": target,
                "binary": binary,
            }))?
        );
    } else {
        ui::kv("rift", version);
        ui::kv("target", target);
        if let Some(b) = binary {
            ui::kv("binary", b);
        }
        ui::spacer();
        ui::note("Run `rift update` to check for newer versions.");
    }

    Ok(())
}

/// Short platform label matching the release archive naming
/// (`rift-{version}-{label}.tar.gz`). Returns `unknown-{os}-{arch}` rather
/// than panicking on unrecognized platforms — `rift version` should always
/// print something useful even on a host that can't self-update.
pub(crate) fn host_target() -> String {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => "linux-x86_64".to_string(),
        ("linux", "aarch64") => "linux-arm64".to_string(),
        // Intel Macs run the arm64 binary under Rosetta 2; the install
        // script (and `rift update`) downloads `macos-arm64` for both.
        ("macos", "x86_64" | "aarch64") => "macos-arm64".to_string(),
        ("windows", "x86_64") => "windows-x86_64".to_string(),
        (os, arch) => format!("unknown-{os}-{arch}"),
    }
}
