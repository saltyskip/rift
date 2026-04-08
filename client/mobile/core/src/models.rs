use serde::Deserialize;

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
pub struct GetLinkResponse {
    pub link_id: String,
    pub ios_deep_link: Option<String>,
    pub android_deep_link: Option<String>,
    pub web_url: Option<String>,
    pub ios_store_url: Option<String>,
    pub android_store_url: Option<String>,
    pub metadata: Option<serde_json::Value>,
}
