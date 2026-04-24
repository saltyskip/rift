//! `rift cancel` — schedule subscription to end at current_period_end.

use crate::context;
use crate::error::CliError;
use crate::ui;

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
            ui::kv("Access ends", format_unix_millis(period_end_ms));
        }
        ui::note(
            "You can reactivate any time before the period ends with `rift subscribe <tier>`.",
        );
    }
    Ok(())
}

fn format_unix_millis(ms: i64) -> String {
    // Duplicated from subscribe.rs to keep the module boundary simple —
    // pulling this into ui.rs would tangle it with every other command.
    use std::time::{Duration, UNIX_EPOCH};
    let secs = (ms / 1000).max(0) as u64;
    let system = UNIX_EPOCH + Duration::from_secs(secs);
    match system.duration_since(UNIX_EPOCH) {
        Ok(d) => {
            let secs = d.as_secs();
            let days = secs / 86_400;
            let (y, m, day) = civil_from_days(days as i64);
            format!("{y:04}-{m:02}-{day:02}")
        }
        Err(_) => format!("{ms}"),
    }
}

fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m as u32, d as u32)
}
