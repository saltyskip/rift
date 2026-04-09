use crate::config::StoredConfig;
use crate::error::CliError;
use crate::ui;

pub async fn run(target: String, json: bool) -> Result<(), CliError> {
    let link_id = target.rsplit('/').next().unwrap_or(&target).to_string();
    let config = StoredConfig::load().ok();
    let client = match config {
        Some(config) => {
            rift_client_core::RiftClient::with_secret_key(config.secret_key, Some(config.base_url))
        }
        None => rift_client_core::RiftClient::anonymous(None),
    };

    let link = client.resolve_link(&link_id).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&link)?);
    } else {
        ui::heading(
            "Link Preview",
            "How Rift will resolve this link across platforms.",
        );
        ui::spacer();
        ui::kv("Link ID", &link.link_id);
        ui::section("Destinations");
        ui::badge(
            "WEB",
            "Desktop / Web",
            &link
                .web_url
                .clone()
                .unwrap_or_else(|| "No web destination configured".to_string()),
            link.web_url.is_some(),
        );
        ui::badge(
            "IOS",
            "iOS",
            &link
                .ios_deep_link
                .clone()
                .or(link.ios_store_url.clone())
                .unwrap_or_else(|| "No iOS destination configured".to_string()),
            link.ios_deep_link.is_some() || link.ios_store_url.is_some(),
        );
        ui::badge(
            "AND",
            "Android",
            &link
                .android_deep_link
                .clone()
                .or(link.android_store_url.clone())
                .unwrap_or_else(|| "No Android destination configured".to_string()),
            link.android_deep_link.is_some() || link.android_store_url.is_some(),
        );
    }

    Ok(())
}
