//! `rift links stats <link-ids>` — funnel stats for one or more links.
//!
//! Wraps `GET /v1/analytics/stats`. Accepts a comma-separated list of link
//! IDs in a single positional argument so the shape matches the API; users
//! who want a quick check on one link just pass the bare ID.

use crate::context;
use crate::error::CliError;
use crate::ui;

pub struct Args {
    pub link_ids: String,
    pub from: Option<String>,
    pub to: Option<String>,
    pub credit: Option<String>,
    pub json: bool,
}

pub async fn run(args: Args) -> Result<(), CliError> {
    let client = context::authenticated_client()?;

    // Same parse the server does: split, trim, drop empties. Done
    // client-side so we can give a useful error before round-tripping.
    let link_ids: Vec<String> = args
        .link_ids
        .split(',')
        .filter_map(|s| {
            let trimmed = s.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        })
        .collect();
    if link_ids.is_empty() {
        return Err(CliError::General(
            "Pass at least one link ID (comma-separated for multiple).".into(),
        ));
    }

    let stats = client
        .get_funnel_stats(
            &link_ids,
            args.from.as_deref(),
            args.to.as_deref(),
            args.credit.as_deref(),
        )
        .await?;

    if args.json {
        println!("{}", serde_json::to_string_pretty(&stats)?);
        return Ok(());
    }

    ui::heading(
        "Link Stats",
        &format!("{} → {} ({})", stats.from, stats.to, stats.credit),
    );
    ui::spacer();
    ui::kv("Links", stats.link_ids.join(", "));

    ui::section("Funnel");
    ui::kv("Clicks", stats.funnel.clicks.to_string());

    ui::section("New users");
    ui::kv("  Installed", stats.funnel.new_users.installed.to_string());
    ui::kv(
        "  Identified",
        stats.funnel.new_users.identified.to_string(),
    );

    ui::section("Returning users");
    ui::kv(
        "  Reinstalled",
        stats.funnel.returning_users.reinstalled.to_string(),
    );
    ui::kv(
        "  New device",
        stats.funnel.returning_users.new_device.to_string(),
    );
    ui::kv(
        "  Engaged",
        stats.funnel.returning_users.engaged.to_string(),
    );

    if stats.funnel.conversions.is_empty() {
        ui::section("Conversions");
        ui::note("No conversion events in this window.");
    } else {
        ui::section("Conversions");
        for (event, count) in &stats.funnel.conversions {
            ui::kv(&format!("  {event}"), count.to_string());
        }
    }

    Ok(())
}
