//! `rift subscribe <tier>` — open Stripe Checkout in a browser and poll the
//! server until the tenant reflects the new tier.

use std::time::{Duration, Instant};

use rift_client_core::billing::PlanTier;

use crate::context;
use crate::error::CliError;
use crate::ui;
use crate::util::format_date_ymd;

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
                    ui::kv("Renews", format_date_ymd(period_end_ms));
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
