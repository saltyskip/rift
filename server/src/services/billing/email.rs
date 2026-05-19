//! Billing-flow emails.
//!
//! The welcome email lives in the billing slice rather than in
//! `core/email.rs` because the copy and CTA shape is billing-specific. The
//! low-level Resend primitive (`core::email::send_email`) is reused.

use crate::core::email;
use crate::services::auth::tenants::models::PlanTier;

/// Welcome email after first successful paid checkout. Contains the one-shot
/// API key, install command, and a link to the Stripe Billing Portal flow.
pub async fn send_welcome(
    resend_api_key: &str,
    from: &str,
    to: &str,
    api_key: &str,
    tier: PlanTier,
    marketing_url: &str,
) -> Result<(), String> {
    let label = tier_label(tier);
    let subject = format!("Welcome to Rift {label} — your API key inside");
    let manage_url = format!("{marketing_url}/manage");
    let install_cmd =
        "curl -fsSL https://raw.githubusercontent.com/saltyskip/rift/main/client/cli/install.sh | sh";

    let html = format!(
        r#"<div style="font-family: system-ui, sans-serif; max-width: 560px; margin: 0 auto; padding: 40px 20px;">
            <h2 style="margin-bottom: 8px;">Welcome to Rift {label}</h2>
            <p style="color: #52525b;">Payment confirmed. Here's what you need to get started.</p>

            <h3 style="margin-top: 32px; font-size: 14px; text-transform: uppercase; letter-spacing: 0.08em; color: #71717a;">Your API key</h3>
            <p style="color: #71717a; font-size: 13px; margin-top: 8px;">Save this now — we'll never show it again.</p>
            <pre style="background: #18181b; color: #f4f4f5; padding: 16px; border-radius: 6px; font-size: 13px; overflow-x: auto; margin-top: 8px;"><code>{api_key}</code></pre>

            <h3 style="margin-top: 32px; font-size: 14px; text-transform: uppercase; letter-spacing: 0.08em; color: #71717a;">Install the CLI</h3>
            <pre style="background: #18181b; color: #f4f4f5; padding: 16px; border-radius: 6px; font-size: 13px; overflow-x: auto; margin-top: 8px;"><code>{install_cmd}</code></pre>

            <h3 style="margin-top: 32px; font-size: 14px; text-transform: uppercase; letter-spacing: 0.08em; color: #71717a;">Log in</h3>
            <pre style="background: #18181b; color: #f4f4f5; padding: 16px; border-radius: 6px; font-size: 13px; overflow-x: auto; margin-top: 8px;"><code>rift login
# paste your API key when prompted</code></pre>

            <h3 style="margin-top: 32px; font-size: 14px; text-transform: uppercase; letter-spacing: 0.08em; color: #71717a;">Manage your subscription</h3>
            <p style="color: #52525b; font-size: 14px;">Update your card, download invoices, or cancel anytime at <a href="{manage_url}" style="color: #0d9488;">{manage_url}</a>.</p>

            <hr style="border: none; border-top: 1px solid #e4e4e7; margin: 40px 0 24px;" />
            <p style="color: #a1a1aa; font-size: 12px;">Rift — Deep links for humans and agents</p>
        </div>"#
    );

    email::send_email(resend_api_key, from, to, &subject, &html).await
}

// ── Helpers ──

fn tier_label(tier: PlanTier) -> &'static str {
    match tier {
        PlanTier::Pro => "Pro",
        PlanTier::Business => "Business",
        PlanTier::Scale => "Scale",
        PlanTier::Free => "Free",
    }
}
