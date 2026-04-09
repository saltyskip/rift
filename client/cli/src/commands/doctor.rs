use crate::context;
use crate::error::CliError;
use crate::ui;
use rift_client_core::error::RiftClientError;

struct CheckRow<'a> {
    done: bool,
    title: &'a str,
    description: &'a str,
}

pub async fn run(json: bool) -> Result<(), CliError> {
    let client = context::authenticated_client()?;

    // Verify the key works before running diagnostics.
    let domains = match client.list_domains().await {
        Ok(d) => d,
        Err(RiftClientError::Api { status: 401, .. }) => {
            return Err(CliError::AuthFailed);
        }
        Err(e) => return Err(e.into()),
    };

    let apps = client.list_apps().await?;
    let publishable_keys = client.list_publishable_keys().await?;

    let has_verified_primary_domain = domains
        .domains
        .iter()
        .any(|domain| domain.verified && domain.role == "primary");
    let has_verified_alternate_domain = domains
        .domains
        .iter()
        .any(|domain| domain.verified && domain.role == "alternate");
    let has_ios_app = apps.apps.iter().any(|app| app.platform == "ios");
    let has_android_app = apps.apps.iter().any(|app| app.platform == "android");
    let android_missing_fingerprints = apps.apps.iter().any(|app| {
        app.platform == "android"
            && app
                .sha256_fingerprints
                .as_ref()
                .map(|fingerprints| fingerprints.is_empty())
                .unwrap_or(true)
    });
    let has_publishable_key = !publishable_keys.keys.is_empty();

    let mut blockers = Vec::new();
    let mut warnings = Vec::new();
    let mut next_steps = Vec::new();

    if !has_publishable_key {
        blockers.push(
            "No publishable key exists yet. Create one after verifying a domain.".to_string(),
        );
        next_steps.push("Create a publishable key after domain verification.".to_string());
    }

    if !has_verified_primary_domain {
        warnings.push("No verified primary domain is configured. Shared-domain links work, but custom slugs and branded links are unavailable.".to_string());
        next_steps.push("Run `rift domains setup` to add a primary domain.".to_string());
    }

    if has_ios_app && !has_verified_alternate_domain {
        warnings.push("No verified alternate domain is configured. iOS Open in App flows will be less reliable without a cross-domain trampoline.".to_string());
        next_steps.push(
            "Add an alternate domain for production-grade iOS open-in-app flows.".to_string(),
        );
    }

    if !has_ios_app && !has_android_app {
        warnings.push(
            "No mobile apps are registered yet. Deep link association files cannot be served."
                .to_string(),
        );
        next_steps.push("Run `rift apps add` to register an iOS or Android app.".to_string());
    }

    if android_missing_fingerprints {
        blockers.push("At least one Android app is missing SHA-256 fingerprints. Android App Links will not verify correctly.".to_string());
        next_steps.push(
            "Update Android app registration with signing certificate fingerprints.".to_string(),
        );
    }

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "has_verified_primary_domain": has_verified_primary_domain,
                "has_verified_alternate_domain": has_verified_alternate_domain,
                "has_ios_app": has_ios_app,
                "has_android_app": has_android_app,
                "has_publishable_key": has_publishable_key,
                "blockers": blockers,
                "warnings": warnings,
                "next_steps": next_steps,
            }))?
        );
    } else {
        let checks = [
            CheckRow {
                done: true,
                title: "Machine connected",
                description:
                    "This machine can create and manage links with your `rl_live_...` key.",
            },
            CheckRow {
                done: true,
                title: "Shared-domain links",
                description: "You can create and test links on Rift's shared domain right now.",
            },
            CheckRow {
                done: has_verified_primary_domain,
                title: "Branded domain",
                description: if has_verified_primary_domain {
                    "Custom domains and branded links are ready."
                } else {
                    "Adds your own domain for branded links and custom slugs."
                },
            },
            CheckRow {
                done: has_publishable_key,
                title: "Client tracking key",
                description: if has_publishable_key {
                    "Client-safe SDK key is available for click tracking and attribution."
                } else {
                    "Enables SDK-based click tracking and attribution flows."
                },
            },
            CheckRow {
                done: has_ios_app || has_android_app,
                title: "App association",
                description: if has_ios_app || has_android_app {
                    "At least one mobile app is registered for deep link association."
                } else {
                    "Registers your iOS or Android app so deep link association files can be served."
                },
            },
            CheckRow {
                done: has_ios_app && has_verified_alternate_domain,
                title: "iOS open-in-app reliability",
                description: if has_ios_app && has_verified_alternate_domain {
                    "Cross-domain trampoline setup is ready for stronger iOS open-in-app flows."
                } else if has_ios_app {
                    "An alternate domain improves iOS Open in App behavior."
                } else {
                    "Add an iOS app first, then an alternate domain for stronger Open in App flows."
                },
            },
            CheckRow {
                done: has_android_app && !android_missing_fingerprints,
                title: "Android App Links verification",
                description: if has_android_app && !android_missing_fingerprints {
                    "Android fingerprints are configured correctly."
                } else if has_android_app {
                    "Add SHA-256 signing fingerprints so Android App Links verify correctly."
                } else {
                    "Add an Android app first, then configure SHA-256 fingerprints for App Links."
                },
            },
        ];

        let ready_count = checks.iter().filter(|check| check.done).count();
        let total_count = checks.len();

        ui::heading(
            "Rift Doctor",
            &format!("Progress: {ready_count}/{total_count} capabilities unlocked"),
        );
        ui::note("Use this report to see what you can do now and what to unlock next.");

        ui::section("Things You Can Do Now");
        for check in checks.iter().filter(|check| check.done) {
            ui::badge("DONE", check.title, check.description, true);
        }

        ui::section("Things You Can Unlock Next");
        for check in checks.iter().filter(|check| !check.done) {
            ui::badge("NEXT", check.title, check.description, false);
        }

        if !next_steps.is_empty() {
            ui::section("What To Do Next");
            ui::numbered(1, &next_steps[0]);

            if next_steps.len() > 1 {
                for (index, step) in next_steps.iter().skip(1).enumerate() {
                    ui::numbered(index + 2, step);
                }
            }
        } else {
            ui::section("What To Do Next");
            ui::note("You're in a good place.");
            ui::code_line("rift links create");
            ui::code_line("rift links test <id>");
        }

        let mut notes = Vec::new();
        notes.extend(blockers);
        notes.extend(warnings);
        if !notes.is_empty() {
            ui::section("Why These Are Next");
            for note in &notes {
                ui::bullet(note);
            }
        }
    }

    Ok(())
}
