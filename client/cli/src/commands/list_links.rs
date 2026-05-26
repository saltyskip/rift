//! `rift links list` — paginated list of links on this tenant.
//!
//! Server pagination is cursor-based: each response carries an optional
//! `next_cursor` that the caller passes back via `--cursor` to fetch the
//! next page. We pass that hint through to the human-readable output so
//! someone working interactively doesn't have to remember the flag name.

use crate::context;
use crate::error::CliError;
use crate::ui;

pub async fn run(limit: Option<i64>, cursor: Option<String>, json: bool) -> Result<(), CliError> {
    let client = context::authenticated_client()?;
    let resp = client.list_links(limit, cursor.as_deref()).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&resp)?);
        return Ok(());
    }

    if resp.links.is_empty() {
        ui::section("Links");
        ui::note("No links yet. Create one with `rift links create`.");
        return Ok(());
    }

    ui::section(&format!("Links ({})", resp.links.len()));
    for link in &resp.links {
        ui::bullet(format!("{}  {}", link.link_id, link.url));
        if let Some(web) = &link.web_url {
            ui::kv("  web", web);
        }
        if let Some(ios) = &link.ios_deep_link {
            ui::kv("  ios", ios);
        }
        if let Some(android) = &link.android_deep_link {
            ui::kv("  android", android);
        }
        ui::kv("  created", &link.created_at);
    }

    if let Some(next) = resp.next_cursor {
        ui::spacer();
        ui::note(&format!(
            "More results — run: rift links list --cursor {next}"
        ));
    }

    Ok(())
}
