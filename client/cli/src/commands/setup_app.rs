use dialoguer::{Input, Select};

use crate::context;
use crate::error::CliError;
use crate::ui;

pub struct Args {
    pub platform: Option<String>,
    pub bundle_id: Option<String>,
    pub team_id: Option<String>,
    pub package_name: Option<String>,
    pub sha256_fingerprints: Option<Vec<String>>,
    pub app_name: Option<String>,
    pub icon_url: Option<String>,
    pub theme_color: Option<String>,
    pub json: bool,
}

pub async fn run(args: Args) -> Result<(), CliError> {
    let client = context::authenticated_client()?;

    let platform = match args.platform {
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
            bundle_id: Some(match args.bundle_id {
                Some(value) => value,
                None => Input::with_theme(&ui::theme())
                    .with_prompt("iOS bundle id")
                    .interact_text()?,
            }),
            team_id: Some(match args.team_id {
                Some(value) => value,
                None => Input::with_theme(&ui::theme())
                    .with_prompt("Apple team id")
                    .interact_text()?,
            }),
            package_name: None,
            sha256_fingerprints: None,
            app_name: args.app_name,
            icon_url: args.icon_url,
            theme_color: args.theme_color,
        }
    } else {
        let sha256_fingerprints = match args.sha256_fingerprints {
            Some(values) if !values.is_empty() => Some(values),
            _ => Some(vec![Input::with_theme(&ui::theme())
                .with_prompt("Android SHA-256 fingerprint")
                .interact_text()?]),
        };
        rift_client_core::apps::CreateAppRequest {
            platform,
            bundle_id: None,
            team_id: None,
            package_name: Some(match args.package_name {
                Some(value) => value,
                None => Input::with_theme(&ui::theme())
                    .with_prompt("Android package name")
                    .interact_text()?,
            }),
            sha256_fingerprints,
            app_name: args.app_name,
            icon_url: args.icon_url,
            theme_color: args.theme_color,
        }
    };

    let app = client.create_app(&request).await?;
    if args.json {
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
        ui::code_line("rift links create");
    }
    Ok(())
}
