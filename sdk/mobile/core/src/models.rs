use serde::{Deserialize, Serialize};

// ── Request types (sent to the API) ──

#[derive(Debug, Serialize)]
pub struct ClickRequest {
    pub link_id: String,
}

#[derive(Debug, Serialize)]
pub struct AttributionRequest {
    pub link_id: String,
    pub install_id: String,
    pub app_version: String,
}

// ── Response types (received from the API) ──

#[derive(Debug, Deserialize)]
pub struct ClickResponse {
    pub link_id: String,
    pub platform: String,
    pub ios_deep_link: Option<String>,
    pub android_deep_link: Option<String>,
    pub web_url: Option<String>,
    pub ios_store_url: Option<String>,
    pub android_store_url: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct AttributionApiResponse {
    pub success: bool,
}

#[derive(Debug, Deserialize)]
pub struct ApiErrorBody {
    pub error: String,
    #[serde(default)]
    pub code: Option<String>,
}
