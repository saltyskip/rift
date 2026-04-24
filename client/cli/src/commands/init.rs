use console::style;
use dialoguer::{Input, Password};

use crate::config::StoredConfig;
use crate::error::CliError;
use crate::ui;
use crate::util;

const RIFT_WORDMARK: &str = r#"
██████╗ ██╗███████╗████████╗
██╔══██╗██║██╔════╝╚══██╔══╝
██████╔╝██║█████╗     ██║
██╔══██╗██║██╔══╝     ██║
██║  ██║██║██║        ██║
╚═╝  ╚═╝╚═╝╚═╝        ╚═╝
"#;

pub fn looks_like_email(value: &str) -> bool {
    let trimmed = value.trim();
    let Some((local, domain)) = trimmed.split_once('@') else {
        return false;
    };

    !local.is_empty()
        && !domain.is_empty()
        && !domain.starts_with('.')
        && !domain.ends_with('.')
        && domain.contains('.')
        && !trimmed.contains(char::is_whitespace)
}

pub async fn run(
    email: Option<String>,
    base_url: Option<String>,
    json: bool,
) -> Result<(), CliError> {
    let base_url = base_url.unwrap_or_else(|| "https://api.riftl.ink".to_string());

    if !json {
        println!("{}", style(RIFT_WORDMARK).cyan().bold());
        ui::heading("Deep links for humans and agents.", "");
        ui::section("This Setup Will");
        ui::numbered(1, "Create or verify your Rift account");
        ui::numbered(2, "Connect this machine to your `rl_live_...` API key");
        ui::numbered(
            3,
            "Optionally create a first test link on the shared Rift domain",
        );
        ui::spacer();
        ui::kv("API base", &base_url);
        ui::spacer();
    }

    let email = match email {
        Some(email) => {
            if !looks_like_email(&email) {
                return Err("Please provide a valid email address.".into());
            }
            email
        }
        None => Input::with_theme(&ui::theme())
            .with_prompt("Email")
            .validate_with(|input: &String| -> Result<(), &str> {
                looks_like_email(input)
                    .then_some(())
                    .ok_or("Enter an email like name@example.com")
            })
            .interact_text()?,
    };

    let anonymous = rift_client_core::RiftClient::anonymous(Some(base_url.clone()));
    let signup = anonymous.signup(email.clone()).await?;

    if !json {
        ui::success(&format!("We sent a verification email to {email}."));
        ui::note(&signup.message);
        ui::section("Next");
        ui::numbered(1, "Open the verification link from your email");
        ui::numbered(2, "Finish verification in the browser");
        ui::numbered(3, "Copy the `rl_live_...` key shown once on that page");
        ui::numbered(4, "Paste it here so this machine can keep using Rift");
        ui::spacer();
    }

    let secret_key = Password::with_theme(&ui::theme())
        .with_prompt("Paste your `rl_live_...` API key")
        .with_confirmation("Confirm API key", "Keys did not match")
        .interact()?;

    let stored = StoredConfig {
        secret_key: secret_key.clone(),
        base_url: base_url.clone(),
    };
    stored.save().map_err(CliError::General)?;
    let config_path = StoredConfig::path().map_err(CliError::General)?;

    let create_starter = ui::choose(
        "Create a first test link on the shared Rift domain?",
        "Yes, create a starter link",
        "No, finish without a link",
        true,
    )?;

    let mut starter_link = None;
    if create_starter {
        if !json {
            ui::section("Starter Link");
            ui::note("Let's create a quick test link so you can see Rift working immediately.");
            ui::note(
                "This first link will use the shared Rift domain and send web users to your URL.",
            );
            ui::note(
                "Use a real page you want to test, like your homepage, docs, or support page.",
            );
            ui::spacer();
        }
        let web_url = Input::with_theme(&ui::theme())
            .with_prompt("Test destination URL")
            .validate_with(|input: &String| -> Result<(), &str> {
                (!input.trim().is_empty())
                    .then_some(())
                    .ok_or("Enter a destination like bitcoin.com or https://bitcoin.com")
            })
            .interact_text()?;
        let web_url = util::normalize_web_url(&web_url);
        let secret_client = rift_client_core::RiftClient::with_secret_key(
            secret_key.clone(),
            Some(base_url.clone()),
        );
        let created = secret_client
            .create_link(&rift_client_core::links::CreateLinkRequest {
                custom_id: None,
                ios_deep_link: None,
                android_deep_link: None,
                web_url: Some(web_url),
                ios_store_url: None,
                android_store_url: None,
                metadata: None,
                agent_context: None,
                social_preview: None,
            })
            .await?;
        starter_link = Some(created);
    }

    if json {
        println!(
            "{}",
            serde_json::json!({
                "base_url": base_url,
                "configured": true,
                "starter_link": starter_link,
            })
        );
    } else {
        ui::success("This machine is now connected to Rift.");
        ui::kv("Saved config", config_path.display().to_string());
        if let Some(link) = starter_link {
            ui::section("Your First Test Link");
            ui::kv("URL", &link.url);
            ui::section("Try It Now");
            ui::bullet("Open the URL above in a browser or on your phone");
            ui::code_line(format!("rift links test {}", link.link_id));
        }
        ui::section("What To Do Next");
        ui::code_line("rift doctor");
        ui::note("Check what is still missing for production.");
        ui::code_line("rift links create");
        ui::note("Create another link with your own destinations.");
        ui::code_line("rift apps add");
        ui::note("Connect iOS or Android app metadata.");
        ui::code_line("rift domains setup");
        ui::note("Add a branded custom domain.");

        // Nudge Free-tier users toward a paid plan. Best-effort: if the
        // server call fails, we silently skip the nudge rather than derail
        // the success path.
        let key_client =
            rift_client_core::RiftClient::with_secret_key(secret_key, Some(base_url.clone()));
        if let Ok(status) = key_client.billing_status().await {
            if matches!(
                status.effective_tier,
                rift_client_core::billing::PlanTier::Free
            ) {
                ui::section("Ready for Production?");
                ui::code_line("rift subscribe pro");
                ui::note("Unlock higher limits and longer analytics retention.");
            }
        }
    }

    Ok(())
}
