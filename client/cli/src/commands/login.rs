use dialoguer::Password;

use crate::config::StoredConfig;
use crate::error::CliError;
use crate::ui;

pub async fn run(base_url: Option<String>, json: bool) -> Result<(), CliError> {
    let base_url = base_url.unwrap_or_else(|| "https://api.riftl.ink".to_string());

    if !json {
        ui::heading(
            "Rift Login",
            "Connect this machine with an existing API key.",
        );
        ui::spacer();
    }

    let secret_key = Password::with_theme(&ui::theme())
        .with_prompt("Paste your `rl_live_...` API key")
        .with_confirmation("Confirm API key", "Keys did not match")
        .interact()?;

    // Verify the key actually works before saving.
    let client =
        rift_client_core::RiftClient::with_secret_key(secret_key.clone(), Some(base_url.clone()));
    client.list_users().await?;

    let stored = StoredConfig {
        secret_key,
        base_url: base_url.clone(),
    };
    stored.save().map_err(CliError::General)?;

    if json {
        println!(
            "{}",
            serde_json::json!({
                "base_url": base_url,
                "configured": true,
            })
        );
    } else {
        let config_path = StoredConfig::path().map_err(CliError::General)?;
        ui::success("This machine is now connected to Rift.");
        ui::kv("Saved config", config_path.display().to_string());
    }

    Ok(())
}
