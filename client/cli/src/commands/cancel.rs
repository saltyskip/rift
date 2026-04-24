//! `rift cancel` — schedule subscription to end at current_period_end.

use crate::context;
use crate::error::CliError;
use crate::ui;
use crate::util::format_date_ymd;

pub async fn run(json: bool) -> Result<(), CliError> {
    let client = context::authenticated_client()?;

    if !json {
        let confirmed = ui::choose(
            "Cancel your Rift subscription at the end of the current period?",
            "Yes, cancel at period end",
            "No, keep my subscription",
            false,
        )?;
        if !confirmed {
            ui::note("Cancelled — your subscription is untouched.");
            return Ok(());
        }
    }

    let resp = client.cancel_subscription().await?;

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "status": resp.status,
                "current_period_end": resp.current_period_end,
            }))?
        );
    } else {
        ui::success(&format!("Scheduled: {}", resp.status));
        if let Some(period_end_ms) = resp.current_period_end {
            ui::kv("Access ends", format_date_ymd(period_end_ms));
        }
        ui::note(
            "You can reactivate any time before the period ends with `rift subscribe <tier>`.",
        );
    }
    Ok(())
}
