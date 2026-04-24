//! `rift billing` — show current plan, effective tier, limits, renewal date.

use crate::context;
use crate::error::CliError;
use crate::ui;

pub async fn run(json: bool) -> Result<(), CliError> {
    let client = context::authenticated_client()?;
    let status = client.billing_status().await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&status)?);
        return Ok(());
    }

    ui::heading("Billing", "Current plan + limits for this tenant");
    ui::spacer();
    ui::kv("Plan tier", status.plan_tier.as_slug());
    if status.effective_tier != status.plan_tier {
        ui::kv("Effective tier", status.effective_tier.as_slug());
    }
    ui::kv(
        "Billing method",
        match status.billing_method {
            rift_client_core::billing::BillingMethod::Free => "free",
            rift_client_core::billing::BillingMethod::Stripe => "stripe",
            rift_client_core::billing::BillingMethod::X402 => "x402",
        },
    );
    ui::kv(
        "Status",
        match status.status {
            rift_client_core::billing::SubscriptionStatus::Active => "active",
            rift_client_core::billing::SubscriptionStatus::PastDue => "past_due",
            rift_client_core::billing::SubscriptionStatus::Canceled => "canceled",
        },
    );
    if status.comp_active {
        ui::kv("Comp overlay", "active");
    }
    if let Some(period_end) = status.current_period_end {
        ui::kv("Period ends", format_unix_millis(period_end));
    }

    ui::section("Limits");
    ui::kv("Links", fmt_limit(status.limits.max_links));
    ui::kv(
        "Events / month",
        fmt_limit(status.limits.max_events_per_month),
    );
    ui::kv("Domains", fmt_limit(status.limits.max_domains));
    ui::kv("Team members", fmt_limit(status.limits.max_team_members));
    ui::kv("Webhooks", fmt_limit(status.limits.max_webhooks));
    ui::kv("Analytics retention", status.limits.analytics_retention);

    Ok(())
}

fn fmt_limit(v: Option<u64>) -> String {
    match v {
        Some(n) => n.to_string(),
        None => "unlimited".to_string(),
    }
}

fn format_unix_millis(ms: i64) -> String {
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
