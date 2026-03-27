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
    #[schema(example = false)]
    pub active: bool,
}
