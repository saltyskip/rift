use mongodb::bson::{oid::ObjectId, DateTime};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum WebhookEventType {
    Click,
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

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateWebhookRequest {
    pub url: String,
    pub events: Vec<WebhookEventType>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CreateWebhookResponse {
    pub id: String,
    pub url: String,
    pub events: Vec<WebhookEventType>,
    pub secret: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct WebhookDetail {
    pub id: String,
    pub url: String,
    pub events: Vec<WebhookEventType>,
    pub active: bool,
    pub created_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ListWebhooksResponse {
    pub webhooks: Vec<WebhookDetail>,
}
