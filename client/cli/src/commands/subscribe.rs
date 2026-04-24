//! `rift subscribe <tier>` — open Stripe Checkout in a browser and poll the
//! server until the tenant reflects the new tier.

use std::time::{Duration, Instant};

use rift_client_core::billing::PlanTier;

use crate::context;
use crate::error::CliError;
use crate::ui;

const POLL_INTERVAL_SECS: u64 = 2;
const POLL_TIMEOUT_SECS: u64 = 180;

pub async fn run(tier_str: String, json: bool) -> Result<(), CliError> {
    let Some(tier) = PlanTier::parse_paid(&tier_str) else {
        return Err(format!("tier must be one of pro, business, scale (got {tier_str})").into());
    };

    let client = context::authenticated_client()?;

    // If the tenant is already on this tier, short-circuit — no sense opening
    // Stripe for a no-op upgrade.
    if let Ok(status) = client.billing_status().await {
        if status.effective_tier == tier {
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "status": "already_subscribed",
                        "tier": tier.as_slug(),
                    }))?
                );
            } else {
                ui::success(&format!("Already on {}.", tier.as_slug()));
            }
            return Ok(());
        }
    }

    let session = client.create_checkout(tier).await?;

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "checkout_url": session.checkout_url,
            }))?
        );
        return Ok(());
    }

    ui::note(&format!("Opening {} in your browser", session.checkout_url));
    if let Err(e) = webbrowser::open(&session.checkout_url) {
        ui::warning(&format!(
            "Couldn't open browser automatically ({e}). Visit the URL above manually."
        ));
    }

    let pb = ui::spinner("Waiting for payment confirmation…");
    let started = Instant::now();
    loop {
        tokio::time::sleep(Duration::from_secs(POLL_INTERVAL_SECS)).await;
        match client.billing_status().await {
            Ok(status) if status.effective_tier == tier => {
                pb.finish_and_clear();
                ui::success(&format!("✓ Upgraded to {}.", tier.as_slug()));
                if let Some(period_end_ms) = status.current_period_end {
                    ui::kv("Renews", format_unix_millis(period_end_ms));
                }
                ui::section("Next");
                ui::code_line("rift billing");
                ui::note("See your limits and renewal date anytime.");
                return Ok(());
            }
            Ok(_) => {}
            Err(e) => {
                // Transient errors shouldn't kill the polling loop — log and
                // keep going until the timeout.
                pb.set_message(format!("Retrying… ({e})"));
            }
        }
        if started.elapsed() > Duration::from_secs(POLL_TIMEOUT_SECS) {
            pb.finish_and_clear();
            ui::warning(
                "Still waiting — if you completed payment, run `rift billing` in a moment.",
            );
            return Ok(());
        }
    }
}

fn format_unix_millis(ms: i64) -> String {
    // Avoid pulling in chrono for one format call — stdlib time is fine.
    // Fall back to the raw epoch if formatting somehow fails.
    use std::time::UNIX_EPOCH;
    let secs = (ms / 1000).max(0) as u64;
    let system = UNIX_EPOCH + Duration::from_secs(secs);
    match system.duration_since(UNIX_EPOCH) {
        Ok(d) => {
            let secs = d.as_secs();
            let days = secs / 86_400;
            // Convert days since epoch (1970-01-01) to YYYY-MM-DD.
            let (y, m, day) = civil_from_days(days as i64);
            format!("{y:04}-{m:02}-{day:02}")
        }
        Err(_) => format!("{ms}"),
    }
}

/// Howard Hinnant's civil_from_days — stdlib-compatible date decomposition.
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
