use serde::{Deserialize, Serialize};

use crate::error::RiftClientError;
use crate::RiftClient;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum WebhookEventType {
    Click,
    Attribution,
}

#[derive(Debug, Serialize)]
pub struct CreateWebhookRequest {
    pub url: String,
    pub events: Vec<WebhookEventType>,
}

#[derive(Debug, Deserialize)]
pub struct WebhookDetail {
    pub id: String,
    pub url: String,
    pub events: Vec<WebhookEventType>,
    pub active: bool,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateWebhookResponse {
    pub id: String,
    pub url: String,
    pub events: Vec<WebhookEventType>,
    pub secret: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct ListWebhooksResponse {
    pub webhooks: Vec<WebhookDetail>,
}

impl RiftClient {
    pub async fn create_webhook(
        &self,
        request: &CreateWebhookRequest,
    ) -> Result<CreateWebhookResponse, RiftClientError> {
        self.post("/v1/webhooks", request, false).await
    }

    pub async fn list_webhooks(&self) -> Result<ListWebhooksResponse, RiftClientError> {
        self.get("/v1/webhooks").await
    }
}
