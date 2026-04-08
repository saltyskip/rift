use dialoguer::{Input, Select};

use crate::config::StoredConfig;
use crate::ui;

#[allow(clippy::too_many_arguments)]
pub async fn run(
    platform: Option<String>,
    bundle_id: Option<String>,
    team_id: Option<String>,
    package_name: Option<String>,
    sha256_fingerprints: Option<Vec<String>>,
    app_name: Option<String>,
    icon_url: Option<String>,
    theme_color: Option<String>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = StoredConfig::load()?;
    let client =
        rift_client_core::RiftClient::with_secret_key(config.secret_key, Some(config.base_url));

    let platform = match platform {
        Some(platform) => platform,
        None => {
            let idx = Select::with_theme(&ui::theme())
                .with_prompt("Platform")
                .items(&["ios", "android"])
                .default(0)
                .interact()?;
            ["ios", "android"][idx].to_string()
        }
    };

    let request = if platform == "ios" {
        rift_client_core::apps::CreateAppRequest {
            platform,
            bundle_id: Some(match bundle_id {
                Some(value) => value,
                None => Input::with_theme(&ui::theme())
                    .with_prompt("iOS bundle id")
                    .interact_text()?,
            }),
            team_id: Some(match team_id {
                Some(value) => value,
                None => Input::with_theme(&ui::theme())
                    .with_prompt("Apple team id")
                    .interact_text()?,
            }),
            package_name: None,
            sha256_fingerprints: None,
            app_name,
            icon_url,
            theme_color,
        }
    } else {
        let sha256_fingerprints = match sha256_fingerprints {
            Some(values) if !values.is_empty() => Some(values),
            _ => Some(vec![Input::with_theme(&ui::theme())
                .with_prompt("Android SHA-256 fingerprint")
                .interact_text()?]),
        };
        rift_client_core::apps::CreateAppRequest {
            platform,
            bundle_id: None,
            team_id: None,
            package_name: Some(match package_name {
                Some(value) => value,
                None => Input::with_theme(&ui::theme())
                    .with_prompt("Android package name")
                    .interact_text()?,
            }),
            sha256_fingerprints,
            app_name,
            icon_url,
            theme_color,
        }
    };

    let app = client.create_app(&request).await?;
    if json {
        println!("{}", serde_json::to_string_pretty(&app)?);
    } else {
        ui::heading(
            "App Registered",
            "Your app metadata is now connected to Rift.",
        );
        ui::spacer();
        ui::kv("Platform", &app.platform);
        ui::kv("App ID", &app.id);
        ui::section("Try This Next");
        ui::code_line("rift doctor");
        ui::code_line("rift create-link");
    }
    Ok(())
}
