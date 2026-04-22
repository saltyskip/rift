//! Minimal Stripe API client for the billing slice.
//!
//! We avoid a full Stripe SDK dep — the surface area we need is small
//! (Checkout Session create + Customer Portal link) and Stripe's REST
//! endpoints are stable and well-documented. Reqwest is already in the
//! tree, so we just POST form-encoded bodies with the secret key.

use crate::services::auth::tenants::repo::PlanTier;

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
