use std::fs;

use crate::config::StoredConfig;
use crate::error::CliError;
use crate::ui;

pub fn run(json: bool) -> Result<(), CliError> {
    let path = StoredConfig::path().map_err(CliError::General)?;
    if !path.exists() {
        if json {
            println!(
                "{}",
                serde_json::json!({ "logged_out": false, "reason": "not_logged_in" })
            );
        } else {
            ui::note("Already logged out. No config file found.");
        }
        return Ok(());
    }

    if !json {
        ui::warning("This will remove the stored API key from this machine.");
        ui::note("You will need your `rl_live_...` key to reconnect. If you don't have it saved elsewhere, you won't be able to recover it.");
        ui::spacer();
        if !ui::choose(
            "Remove the stored API key?",
            "Yes, log out",
            "No, keep it",
            false,
        )? {
            ui::note("Cancelled. Your config is unchanged.");
            return Ok(());
        }
    }

    fs::remove_file(&path).map_err(|e| CliError::General(e.to_string()))?;

    if json {
        println!("{}", serde_json::json!({ "logged_out": true }));
    } else {
        ui::success("Logged out. Config file removed.");
        ui::kv("Removed", path.display().to_string());
    }

    Ok(())
}
