//! Minimal Stripe API client for the billing slice.
//!
//! We avoid a full Stripe SDK dep — the surface area we need is small
//! (Checkout Session create + Customer Portal link + webhook HMAC) and
//! Stripe's REST endpoints are stable and well-documented. Reqwest is
//! already in the tree, so we just POST form-encoded bodies with the
//! secret key. HMAC verification uses the `hmac` crate already pulled in
//! by services/webhooks/dispatcher.rs for outbound signing.

use hmac::{Hmac, Mac};
use sha2::Sha256;

pub use super::models::{
    CheckoutSession, HandoffCheckoutOpts, PortalSession, StripeConfig, StripeError,
    WebhookVerifyError,
};
use crate::services::auth::tenants::repo::PlanTier;

type HmacSha256 = Hmac<Sha256>;

const STRIPE_API_BASE: &str = "https://api.stripe.com/v1";

/// Stripe-Version header pinned for all outbound calls.
///
/// Pinning guards against silent Stripe-side version rollups that could break
/// our deserializers. Our webhook parser already tolerates the 2025+ shape
/// where `current_period_*` moved from the top-level subscription onto
/// individual items, so we track the current Dashboard default here. Bump
/// deliberately after reviewing Stripe's upgrade notes.
const STRIPE_API_VERSION: &str = "2026-03-25.dahlia";

/// Create a Stripe Checkout session for a subscription to the given tier.
/// `client_reference_id` is the tenant id hex string — Stripe echoes it
/// back on the webhook so we can map the subscription to our tenant.
pub async fn create_checkout_session(
    cfg: &StripeConfig,
    tier: PlanTier,
    tenant_id_hex: &str,
) -> Result<CheckoutSession, StripeError> {
    if !cfg.is_configured() {
        return Err(StripeError::NotConfigured);
    }
    let price_id = cfg
        .price_id_for(tier)
        .ok_or(StripeError::MissingPriceId(tier))?;

    // Stripe expects application/x-www-form-urlencoded.
    let params = [
        ("mode", "subscription"),
        ("line_items[0][price]", price_id),
        ("line_items[0][quantity]", "1"),
        ("success_url", &cfg.success_url),
        ("cancel_url", &cfg.cancel_url),
        ("client_reference_id", tenant_id_hex),
        // Let Stripe manage the tax if the account has tax enabled. Safe default.
        ("automatic_tax[enabled]", "false"),
        // Include the tenant id on the subscription metadata too — makes
        // webhook payloads easier to route when `client_reference_id` isn't
        // on a later `customer.subscription.*` event.
        ("subscription_data[metadata][tenant_id]", tenant_id_hex),
    ];

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{STRIPE_API_BASE}/checkout/sessions"))
        .basic_auth(&cfg.secret_key, None::<&str>)
        .header("Stripe-Version", STRIPE_API_VERSION)
        .form(&params)
        .send()
        .await
        .map_err(|e| StripeError::Network(e.to_string()))?;

    let status = resp.status();
    let body = resp
        .text()
        .await
        .map_err(|e| StripeError::Network(e.to_string()))?;

    if !status.is_success() {
        return Err(StripeError::Api(format!("{status}: {body}")));
    }

    serde_json::from_str::<CheckoutSession>(&body).map_err(|e| StripeError::Api(e.to_string()))
}

/// Create a Checkout session for the billing-handoff flow (email magic-link
/// → Stripe Checkout).
pub async fn create_checkout_session_for_handoff(
    cfg: &StripeConfig,
    opts: HandoffCheckoutOpts<'_>,
) -> Result<CheckoutSession, StripeError> {
    if !cfg.is_configured() {
        return Err(StripeError::NotConfigured);
    }
    let price_id = cfg
        .price_id_for(opts.tier)
        .ok_or(StripeError::MissingPriceId(opts.tier))?;

    // Stripe's form encoder accepts repeated keys, so we build the params
    // as a Vec of (&str, &str) rather than a fixed-size array.
    let mut params: Vec<(&str, &str)> = vec![
        ("mode", "subscription"),
        ("line_items[0][price]", price_id),
        ("line_items[0][quantity]", "1"),
        ("success_url", opts.success_url),
        ("cancel_url", opts.cancel_url),
        ("automatic_tax[enabled]", "false"),
    ];
    if let Some(cust) = opts.customer_id {
        params.push(("customer", cust));
    } else if let Some(email) = opts.customer_email {
        params.push(("customer_email", email));
    }
    if let Some(tenant_id) = opts.tenant_id_hex {
        params.push(("client_reference_id", tenant_id));
        params.push(("subscription_data[metadata][tenant_id]", tenant_id));
    }
    if let Some(pending_email) = opts.pending_email {
        params.push(("subscription_data[metadata][pending_email]", pending_email));
    }

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{STRIPE_API_BASE}/checkout/sessions"))
        .basic_auth(&cfg.secret_key, None::<&str>)
        .header("Stripe-Version", STRIPE_API_VERSION)
        .form(&params)
        .send()
        .await
        .map_err(|e| StripeError::Network(e.to_string()))?;

    let status = resp.status();
    let body = resp
        .text()
        .await
        .map_err(|e| StripeError::Network(e.to_string()))?;

    if !status.is_success() {
        return Err(StripeError::Api(format!("{status}: {body}")));
    }

    serde_json::from_str::<CheckoutSession>(&body).map_err(|e| StripeError::Api(e.to_string()))
}

/// Create a Stripe Billing Portal session for an existing customer. The
/// customer manages their subscription on Stripe's hosted UI and returns to
/// `return_url` when done.
pub async fn create_portal_session(
    secret_key: &str,
    customer_id: &str,
    return_url: &str,
) -> Result<PortalSession, StripeError> {
    if secret_key.is_empty() {
        return Err(StripeError::NotConfigured);
    }
    let params = [("customer", customer_id), ("return_url", return_url)];

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{STRIPE_API_BASE}/billing_portal/sessions"))
        .basic_auth(secret_key, None::<&str>)
        .header("Stripe-Version", STRIPE_API_VERSION)
        .form(&params)
        .send()
        .await
        .map_err(|e| StripeError::Network(e.to_string()))?;

    let status = resp.status();
    let body = resp
        .text()
        .await
        .map_err(|e| StripeError::Network(e.to_string()))?;

    if !status.is_success() {
        return Err(StripeError::Api(format!("{status}: {body}")));
    }

    serde_json::from_str::<PortalSession>(&body).map_err(|e| StripeError::Api(e.to_string()))
}

/// Schedule a subscription to end at the current period end. Stripe will
/// keep the subscription active until `current_period_end`, then fire
/// `customer.subscription.deleted` which our webhook handler converts into
/// a tenant downgrade to Free.
pub async fn cancel_subscription_at_period_end(
    secret_key: &str,
    subscription_id: &str,
) -> Result<(), StripeError> {
    if secret_key.is_empty() {
        return Err(StripeError::NotConfigured);
    }
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{STRIPE_API_BASE}/subscriptions/{subscription_id}"))
        .basic_auth(secret_key, None::<&str>)
        .header("Stripe-Version", STRIPE_API_VERSION)
        .form(&[("cancel_at_period_end", "true")])
        .send()
        .await
        .map_err(|e| StripeError::Network(e.to_string()))?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp
            .text()
            .await
            .map_err(|e| StripeError::Network(e.to_string()))?;
        return Err(StripeError::Api(format!("{status}: {body}")));
    }
    Ok(())
}

// ── Webhook signature verification ──

/// Stripe tolerates webhook deliveries up to 5 minutes old. Anything older is
/// assumed to be a replayed / captured request and rejected.
const WEBHOOK_TOLERANCE_SECS: i64 = 300;

/// Verify a Stripe webhook signature over the raw request body.
///
/// Stripe-Signature is of the form `t=TIMESTAMP,v1=SIG[,v0=...,v1=SIG2]`.
/// A single header may contain multiple `v1=` entries (e.g. during rolling
/// secret rotation) — any valid one accepts the event.
///
/// `signing_secret` is the `whsec_...` value from the Stripe dashboard.
/// `now_secs` is the current Unix time — parameterized for testability.
pub fn verify_webhook_signature(
    signing_secret: &str,
    header: &str,
    body: &[u8],
    now_secs: i64,
) -> Result<(), WebhookVerifyError> {
    let mut ts: Option<i64> = None;
    let mut v1_sigs: Vec<&str> = Vec::new();

    for part in header.split(',') {
        let (k, v) = part.split_once('=').ok_or(WebhookVerifyError::BadHeader)?;
        match k.trim() {
            "t" => {
                ts = Some(
                    v.trim()
                        .parse()
                        .map_err(|_| WebhookVerifyError::BadHeader)?,
                );
            }
            "v1" => v1_sigs.push(v.trim()),
            _ => {}
        }
    }

    let ts = ts.ok_or(WebhookVerifyError::BadHeader)?;
    if (now_secs - ts).abs() > WEBHOOK_TOLERANCE_SECS {
        return Err(WebhookVerifyError::TimestampTooOld);
    }

    if v1_sigs.is_empty() {
        return Err(WebhookVerifyError::BadHeader);
    }

    // Signed payload: "TIMESTAMP.BODY"
    let mut mac = HmacSha256::new_from_slice(signing_secret.as_bytes())
        .map_err(|_| WebhookVerifyError::BadHeader)?;
    mac.update(ts.to_string().as_bytes());
    mac.update(b".");
    mac.update(body);
    let expected = mac.finalize().into_bytes();
    let expected_hex = hex::encode(expected);

    // Constant-time compare across all v1 candidates.
    if v1_sigs.iter().any(|sig| {
        // Use the hmac crate's verify_slice via a fresh Mac to get CT compare.
        let decoded = match hex::decode(sig) {
            Ok(b) => b,
            Err(_) => return false,
        };
        let mut mac2 = match HmacSha256::new_from_slice(signing_secret.as_bytes()) {
            Ok(m) => m,
            Err(_) => return false,
        };
        mac2.update(ts.to_string().as_bytes());
        mac2.update(b".");
        mac2.update(body);
        mac2.verify_slice(&decoded).is_ok() && sig.eq_ignore_ascii_case(&expected_hex)
    }) {
        Ok(())
    } else {
        Err(WebhookVerifyError::SignatureMismatch)
    }
}

#[cfg(test)]
#[path = "stripe_client_tests.rs"]
mod tests;
