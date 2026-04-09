use crate::context;
use crate::error::CliError;
use crate::ui;

pub async fn run(json: bool) -> Result<(), CliError> {
    let client = context::authenticated_client()?;
    let users = client.list_users().await?;

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "users": users.users.iter().map(|u| serde_json::json!({
                    "id": u.id,
                    "email": u.email,
                    "verified": u.verified,
                    "is_owner": u.is_owner,
                })).collect::<Vec<_>>(),
            }))?
        );
    } else {
        ui::heading("Rift Account", "Your API key is valid.");
        ui::spacer();
        ui::section("Team Members");
        for user in &users.users {
            let role = if user.is_owner { "owner" } else { "member" };
            let status = if user.verified { "verified" } else { "pending" };
            ui::bullet(format!("{} ({}, {})", user.email, role, status));
        }
    }

    Ok(())
}
