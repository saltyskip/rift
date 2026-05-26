//! `rift team remove <email-or-id>` — remove a team member from this tenant.
//!
//! Accepts either an email address (looked up via `list_users`) or a raw
//! user id. Email-first because that's how humans reach for "remove alice".

use crate::context;
use crate::error::CliError;
use crate::ui;

pub async fn run(who: String, json: bool) -> Result<(), CliError> {
    let client = context::authenticated_client()?;

    // Email or id? Bare ObjectId hex is always 24 lowercase hex chars; an
    // email always contains '@'. Anything else is a typo — error early so
    // the user doesn't see a server-side 404 they have to parse.
    let target = resolve_target(&client, &who).await?;

    if !json {
        ui::warning(&format!(
            "This will remove {} from your team.",
            target.label
        ));
        ui::note("They'll lose access to all links, domains, and keys on this tenant. This cannot be undone.");
        ui::spacer();
        if !ui::choose(
            "Remove this team member?",
            "Yes, remove",
            "No, keep them",
            false,
        )? {
            ui::note("Cancelled. No changes made.");
            return Ok(());
        }
    }

    client.remove_user(&target.id).await?;

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "removed": true,
                "id": target.id,
                "email": target.email,
            }))?
        );
    } else {
        ui::success(&format!("Removed {}", target.label));
    }

    Ok(())
}

struct Target {
    id: String,
    email: Option<String>,
    /// Either email or id, whichever the user provided — used for
    /// confirmation prompts and success messages.
    label: String,
}

async fn resolve_target(
    client: &rift_client_core::RiftClient,
    who: &str,
) -> Result<Target, CliError> {
    if who.contains('@') {
        let users = client.list_users().await?;
        let user = users
            .users
            .into_iter()
            .find(|u| u.email.eq_ignore_ascii_case(who))
            .ok_or_else(|| {
                CliError::General(format!("No team member with email {who}. Run `rift team list` to see who's on this tenant."))
            })?;
        Ok(Target {
            id: user.id,
            label: user.email.clone(),
            email: Some(user.email),
        })
    } else if is_object_id_hex(who) {
        Ok(Target {
            id: who.to_string(),
            email: None,
            label: who.to_string(),
        })
    } else {
        Err(CliError::General(format!(
            "{who} doesn't look like an email or a user id. Pass an email address (alice@example.com) or a 24-char hex id from `rift team list`."
        )))
    }
}

fn is_object_id_hex(s: &str) -> bool {
    s.len() == 24 && s.chars().all(|c| c.is_ascii_hexdigit())
}
