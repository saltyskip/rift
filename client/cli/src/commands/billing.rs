//! `rift billing` — show current plan, effective tier, limits, renewal date.

use crate::context;
use crate::error::CliError;
use crate::ui;
use crate::util::format_date_ymd;

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
        ui::kv("Period ends", format_date_ymd(period_end));
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
