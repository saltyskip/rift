use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ClickEventPayload {
    pub tenant_id: String,
    pub link_id: String,
    pub user_agent: Option<String>,
    pub referer: Option<String>,
    pub platform: String,
    pub country: Option<String>,
    pub city: Option<String>,
    pub region: Option<String>,
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
pub struct WebhookPayload {
    pub event: String,
    pub timestamp: String,
    pub data: serde_json::Value,
}

pub trait WebhookDispatcher: Send + Sync {
    fn dispatch_click(&self, payload: ClickEventPayload);
    fn dispatch_attribution(&self, payload: AttributionEventPayload);
}
