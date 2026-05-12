use mongodb::bson::{oid::ObjectId, DateTime};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum WebhookEventType {
    /// Fired when any user or agent resolves one of your links.
    Click,
    /// Fired when a mobile SDK reports a post-install attribution.
    Attribution,
    /// Fired when a conversion event is ingested via a source webhook.
    /// Carries a stable `event_id` so customer handlers can dedup on retries.
    Conversion,
    /// Fired when `PUT /v1/attribution/identify` successfully binds an
    /// install to a user and a prior attribution exists. Payload carries the
    /// resolved triple `{user_id, link_id, link_metadata}` so receivers can
    /// react (grant entitlements, etc.) without a follow-up Link lookup.
    Identify,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Webhook {
    #[serde(rename = "_id")]
    pub id: ObjectId,
    pub tenant_id: ObjectId,
    pub url: String,
    pub secret: String,
    pub events: Vec<WebhookEventType>,
    pub active: bool,
    pub created_at: DateTime,
}

/// Register a URL to receive real-time HMAC-signed POST requests when clicks or attributions occur.
/// The signing secret is returned once at creation time — save it to verify webhook signatures.
/// Payloads are JSON with `event`, `timestamp`, and `data` fields. Maximum 2 webhooks per tenant.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateWebhookRequest {
    /// HTTPS URL to receive webhook POST requests.
    #[schema(example = "https://api.tablefour.com/webhooks/relay")]
    pub url: String,
    pub events: Vec<WebhookEventType>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CreateWebhookResponse {
    #[schema(example = "665a1b2c3d4e5f6a7b8c9d0e")]
    pub id: String,
    #[schema(example = "https://api.tablefour.com/webhooks/relay")]
    pub url: String,
    pub events: Vec<WebhookEventType>,
    /// HMAC-SHA256 signing secret. Use this to verify webhook payloads. Shown only once at creation time.
    #[schema(example = "whsec_k7J2mN9pQ4rT1vX8yB3cF6gH0")]
    pub secret: String,
    #[schema(example = "2025-06-15T10:30:00Z")]
    pub created_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct WebhookDetail {
    #[schema(example = "665a1b2c3d4e5f6a7b8c9d0e")]
    pub id: String,
    #[schema(example = "https://api.tablefour.com/webhooks/relay")]
    pub url: String,
    pub events: Vec<WebhookEventType>,
    #[schema(example = true)]
    pub active: bool,
    #[schema(example = "2025-06-15T10:30:00Z")]
    pub created_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ListWebhooksResponse {
    pub webhooks: Vec<WebhookDetail>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateWebhookRequest {
    /// Enable / disable delivery. Omit to leave unchanged.
    #[serde(default)]
    #[schema(example = false)]
    pub active: Option<bool>,
    /// Replace the subscribed event types. Omit to leave unchanged; pass
    /// an empty array to reject (must subscribe to at least one event).
    #[serde(default)]
    pub events: Option<Vec<WebhookEventType>>,
}

// ── Errors ──

use crate::services::billing::quota::QuotaError;

#[derive(Debug)]
pub enum WebhookError {
    QuotaExceeded(QuotaError),
    Internal(String),
}

impl From<QuotaError> for WebhookError {
    fn from(err: QuotaError) -> Self {
        WebhookError::QuotaExceeded(err)
    }
}

impl std::fmt::Display for WebhookError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::QuotaExceeded(e) => write!(f, "{e}"),
            Self::Internal(e) => write!(f, "Internal error: {e}"),
        }
    }
}
