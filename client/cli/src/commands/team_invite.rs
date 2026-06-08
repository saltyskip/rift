//! `rift team invite <email>` — send a team-member invite.

use crate::context;
use crate::error::CliError;
use crate::ui;

pub async fn run(email: String, json: bool) -> Result<(), CliError> {
    let client = context::authenticated_client()?;
    let resp = client.invite_user(email).await?;

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "id": resp.id,
                "email": resp.email,
                "status": resp.status,
                "resent": resp.resent,
            }))?
        );
    } else {
        if resp.resent {
            ui::success(&format!("Re-sent invite to {}", resp.email));
            ui::note("Their previous link had expired. A fresh verification email is on its way.");
        } else {
            ui::success(&format!("Invited {}", resp.email));
            ui::note("They'll receive a verification email. Once they confirm, they can sign in at /signin and share this tenant's resources.");
        }
    }

    Ok(())
}
