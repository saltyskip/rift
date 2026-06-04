use std::fs;

use crate::config::StoredConfig;
use crate::error::CliError;
use crate::ui;

pub async fn run(json: bool) -> Result<(), CliError> {
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

    // Loaded up front so we can revoke a server-side session before deleting.
    let config = StoredConfig::load().ok();

    if !json {
        ui::warning("This will remove the stored Rift credentials from this machine.");
        ui::note("You'll need to run `rift login` again to reconnect.");
        ui::spacer();
        if !ui::choose(
            "Remove the stored credentials?",
            "Yes, log out",
            "No, keep it",
            false,
        )? {
            ui::note("Cancelled. Your config is unchanged.");
            return Ok(());
        }
    }

    // Best-effort: revoke the session server-side so the token can't be reused
    // even if the local file is recovered. API keys are revoked from /account,
    // not here.
    let mut session_revoked = false;
    if let Some(config) = &config {
        if let Some(token) = &config.session_token {
            let client = rift_client_core::RiftClient::with_session_token(
                token.clone(),
                Some(config.base_url.clone()),
            );
            session_revoked = client.signout().await.is_ok();
        }
    }

    fs::remove_file(&path).map_err(|e| CliError::General(e.to_string()))?;

    if json {
        println!(
            "{}",
            serde_json::json!({ "logged_out": true, "session_revoked": session_revoked })
        );
    } else {
        ui::success("Logged out. Config file removed.");
        ui::kv("Removed", path.display().to_string());
    }

    Ok(())
}
