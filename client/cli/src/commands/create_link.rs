use dialoguer::Input;

use crate::config::StoredConfig;
use crate::ui;

fn normalize_web_url(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.contains("://") {
        trimmed.to_string()
    } else {
        format!("https://{trimmed}")
    }
}

pub async fn run(
    web_url: Option<String>,
    ios_deep_link: Option<String>,
    android_deep_link: Option<String>,
    ios_store_url: Option<String>,
    android_store_url: Option<String>,
    custom_id: Option<String>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = StoredConfig::load()?;
    let client =
        rift_client_core::RiftClient::with_secret_key(config.secret_key, Some(config.base_url));
    let web_url = match web_url {
        Some(url) => normalize_web_url(&url),
        None => {
            let url: String = Input::with_theme(&ui::theme())
                .with_prompt("Web URL")
                .validate_with(|input: &String| -> Result<(), &str> {
                    (!input.trim().is_empty())
                        .then_some(())
                        .ok_or("Enter a URL like bitcoin.com or https://bitcoin.com")
                })
                .interact_text()?;
            normalize_web_url(&url)
        }
    };

    let created = client
        .create_link(&rift_client_core::links::CreateLinkRequest {
            custom_id,
            ios_deep_link,
            android_deep_link,
            web_url: Some(web_url),
            ios_store_url,
            android_store_url,
            metadata: None,
            agent_context: None,
        })
        .await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&created)?);
    } else {
        ui::heading("Link Created", "Your new Rift link is ready to test.");
        ui::spacer();
        ui::kv("Link ID", &created.link_id);
        ui::kv("URL", &created.url);
        if let Some(expires_at) = created.expires_at {
            ui::kv("Expires", expires_at);
        }
        ui::section("Try This Next");
        ui::code_line(format!("rift test-link {}", created.link_id));
        ui::code_line(&created.url);
    }

    Ok(())
}
