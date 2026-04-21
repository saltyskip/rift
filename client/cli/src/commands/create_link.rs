use dialoguer::Input;

use crate::context;
use crate::error::CliError;
use crate::ui;
use crate::util;

pub struct Args {
    pub web_url: Option<String>,
    pub ios_deep_link: Option<String>,
    pub android_deep_link: Option<String>,
    pub ios_store_url: Option<String>,
    pub android_store_url: Option<String>,
    pub custom_id: Option<String>,
    pub preview_title: Option<String>,
    pub preview_description: Option<String>,
    pub preview_image_url: Option<String>,
    pub json: bool,
}

pub async fn run(args: Args) -> Result<(), CliError> {
    let client = context::authenticated_client()?;
    let web_url = match args.web_url {
        Some(url) => util::normalize_web_url(&url),
        None => {
            let url: String = Input::with_theme(&ui::theme())
                .with_prompt("Web URL")
                .validate_with(|input: &String| -> Result<(), &str> {
                    (!input.trim().is_empty())
                        .then_some(())
                        .ok_or("Enter a URL like bitcoin.com or https://bitcoin.com")
                })
                .interact_text()?;
            util::normalize_web_url(&url)
        }
    };

    let created = client
        .create_link(&rift_client_core::links::CreateLinkRequest {
            custom_id: args.custom_id,
            ios_deep_link: args.ios_deep_link,
            android_deep_link: args.android_deep_link,
            web_url: Some(web_url),
            ios_store_url: args.ios_store_url,
            android_store_url: args.android_store_url,
            metadata: None,
            agent_context: None,
            social_preview: build_social_preview(
                args.preview_title,
                args.preview_description,
                args.preview_image_url,
            ),
        })
        .await?;

    if args.json {
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
        ui::code_line(format!("rift links test {}", created.link_id));
        ui::code_line(&created.url);
    }

    Ok(())
}

fn build_social_preview(
    title: Option<String>,
    description: Option<String>,
    image_url: Option<String>,
) -> Option<rift_client_core::links::SocialPreview> {
    (title.is_some() || description.is_some() || image_url.is_some()).then_some(
        rift_client_core::links::SocialPreview {
            title,
            description,
            image_url,
        },
    )
}
