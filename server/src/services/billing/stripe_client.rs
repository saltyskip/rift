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

use crate::services::auth::tenants::repo::PlanTier;

type HmacSha256 = Hmac<Sha256>;

const STRIPE_API_BASE: &str = "https://api.stripe.com/v1";

#[derive(Debug, Clone)]
pub struct StripeConfig {
    pub secret_key: String,
    pub price_id_pro: String,
    pub price_id_business: String,
    pub price_id_scale: String,
    pub success_url: String,
    pub cancel_url: String,
}

impl StripeConfig {
    pub fn is_configured(&self) -> bool {
        !self.secret_key.is_empty()
    }

    pub fn price_id_for(&self, tier: PlanTier) -> Option<&str> {
        let id = match tier {
            PlanTier::Free => return None,
            PlanTier::Pro => &self.price_id_pro,
            PlanTier::Business => &self.price_id_business,
            PlanTier::Scale => &self.price_id_scale,
        };
        if id.is_empty() {
            None
        } else {
            Some(id)
        }
    }
}

#[derive(Debug)]
pub enum StripeError {
    NotConfigured,
    MissingPriceId(PlanTier),
    Api(String),
    Network(String),
}

impl std::fmt::Display for StripeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotConfigured => write!(f, "Stripe not configured"),
            Self::MissingPriceId(t) => write!(f, "Missing Stripe price ID for tier {t:?}"),
            Self::Api(e) => write!(f, "Stripe API error: {e}"),
            Self::Network(e) => write!(f, "Network error calling Stripe: {e}"),
        }
    }
}

/// Result of creating a Checkout session — the caller redirects the user
/// to `url`.
#[derive(Debug, serde::Deserialize)]
pub struct CheckoutSession {
    pub url: String,
}

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

// ── Webhook signature verification ──

/// Stripe tolerates webhook deliveries up to 5 minutes old. Anything older is
/// assumed to be a replayed / captured request and rejected.
const WEBHOOK_TOLERANCE_SECS: i64 = 300;

#[derive(Debug)]
pub enum WebhookVerifyError {
    /// Signature header missing or malformed.
    BadHeader,
    /// Timestamp older than the tolerance window.
    TimestampTooOld,
    /// No `v1=` component in the header matched the expected signature.
    SignatureMismatch,
}

impl std::fmt::Display for WebhookVerifyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadHeader => write!(f, "malformed Stripe-Signature header"),
            Self::TimestampTooOld => write!(f, "webhook timestamp older than 5 min"),
            Self::SignatureMismatch => write!(f, "signature mismatch"),
        }
    }
}

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
mod tests {
    use super::*;

    fn sign(secret: &str, ts: i64, body: &[u8]) -> String {
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(ts.to_string().as_bytes());
        mac.update(b".");
        mac.update(body);
        hex::encode(mac.finalize().into_bytes())
    }

    #[test]
    fn verifies_correct_signature() {
        let secret = "whsec_test";
        let body = b"{\"id\":\"evt_123\"}";
        let ts = 1_000_000;
        let sig = sign(secret, ts, body);
        let header = format!("t={ts},v1={sig}");
        assert!(verify_webhook_signature(secret, &header, body, ts).is_ok());
    }

    #[test]
    fn rejects_bad_signature() {
        let secret = "whsec_test";
        let body = b"{\"id\":\"evt_123\"}";
        let ts = 1_000_000;
        let sig = sign("whsec_wrong", ts, body);
        let header = format!("t={ts},v1={sig}");
        let err = verify_webhook_signature(secret, &header, body, ts).unwrap_err();
        assert!(matches!(err, WebhookVerifyError::SignatureMismatch));
    }

    #[test]
    fn rejects_altered_body() {
        let secret = "whsec_test";
        let body = b"{\"id\":\"evt_123\"}";
        let ts = 1_000_000;
        let sig = sign(secret, ts, body);
        let header = format!("t={ts},v1={sig}");
        let err =
            verify_webhook_signature(secret, &header, b"{\"id\":\"evt_456\"}", ts).unwrap_err();
        assert!(matches!(err, WebhookVerifyError::SignatureMismatch));
    }

    #[test]
    fn rejects_stale_timestamp() {
        let secret = "whsec_test";
        let body = b"{\"id\":\"evt_123\"}";
        let ts = 1_000_000;
        let sig = sign(secret, ts, body);
        let header = format!("t={ts},v1={sig}");
        // 400 seconds later is outside the 5-min tolerance.
        let err = verify_webhook_signature(secret, &header, body, ts + 400).unwrap_err();
        assert!(matches!(err, WebhookVerifyError::TimestampTooOld));
    }

    #[test]
    fn accepts_multiple_v1_signatures_any_valid() {
        let secret = "whsec_test";
        let body = b"body";
        let ts = 1_000_000;
        let good = sign(secret, ts, body);
        let header = format!("t={ts},v1=badsig000,v1={good}");
        assert!(verify_webhook_signature(secret, &header, body, ts).is_ok());
    }

    #[test]
    fn malformed_header_rejected() {
        let err = verify_webhook_signature("s", "not_a_header", b"body", 0).unwrap_err();
        assert!(matches!(err, WebhookVerifyError::BadHeader));
    }
}
