use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ClickEventPayload {
    pub tenant_id: String,
    pub link_id: String,
    pub user_agent: Option<String>,
    pub referer: Option<String>,
    pub platform: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AttributionEventPayload {
    pub tenant_id: String,
    pub link_id: String,
    pub install_id: String,
    pub app_version: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConversionEventPayload {
    /// Stable unique ID for this event. Customer handlers should use it as an
    /// idempotency key to avoid double-counting on webhook delivery retries.
    pub event_id: String,
    pub tenant_id: String,
    pub source_id: String,
    pub link_id: String,
    pub conversion_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount_cents: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct WebhookPayload {
    pub event: String,
    pub timestamp: String,
    pub data: serde_json::Value,
}

pub trait WebhookDispatcher: Send + Sync {
    fn dispatch_click(&self, payload: ClickEventPayload);
    fn dispatch_attribution(&self, payload: AttributionEventPayload);
    fn dispatch_conversion(&self, payload: ConversionEventPayload);
}
