//! `rift team list` — list team members on this tenant.
//!
//! Overlaps with `rift whoami` (which also prints team members under the
//! account summary). This command exists because `team list` is the
//! intuitive reach and produces cleaner JSON for scripting.

use crate::context;
use crate::error::CliError;
use crate::ui;

pub async fn run(json: bool) -> Result<(), CliError> {
    let client = context::authenticated_client()?;
    let resp = client.list_users().await?;

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "users": resp.users.iter().map(|u| serde_json::json!({
                    "id": u.id,
                    "email": u.email,
                    "verified": u.verified,
                    "status": u.status,
                    "is_owner": u.is_owner,
                    "created_at": u.created_at,
                })).collect::<Vec<_>>(),
            }))?
        );
    } else {
        ui::section("Team Members");
        for user in &resp.users {
            let role = if user.is_owner { "owner" } else { "member" };
            // Prefer the server's lifecycle status (active/pending/expired);
            // fall back to verified for older servers that don't send it.
            let status = if user.status.is_empty() {
                if user.verified {
                    "active"
                } else {
                    "pending"
                }
            } else {
                user.status.as_str()
            };
            ui::bullet(format!("{} ({}, {})", user.email, role, status));
            ui::kv("  id", &user.id);
        }
    }

    Ok(())
}
