use mongodb::bson::{oid::ObjectId, DateTime, Document};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

// ── Database Documents ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Link {
    #[serde(rename = "_id")]
    pub id: ObjectId,
    /// Tenant who owns this link (API key ObjectId).
    pub tenant_id: ObjectId,
    /// Short alphanumeric ID used in URLs (e.g. "ABCD1234").
    pub link_id: String,
    /// iOS deep link URI (e.g. "myapp://product/123").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ios_deep_link: Option<String>,
    /// Android deep link URI.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub android_deep_link: Option<String>,
    /// Web fallback URL for desktop/unknown platforms.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub web_url: Option<String>,
    /// App Store link for iOS.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ios_store_url: Option<String>,
    /// Play Store link for Android.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub android_store_url: Option<String>,
    /// Arbitrary metadata (campaign name, source, etc.).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Document>,
    pub created_at: DateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Click {
    #[serde(rename = "_id")]
    pub id: ObjectId,
    pub tenant_id: ObjectId,
    pub link_id: String,
    pub clicked_at: DateTime,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_agent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub referer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform: Option<String>,
    /// Short hex token for deferred deep linking.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attribution {
    #[serde(rename = "_id")]
    pub id: ObjectId,
    pub tenant_id: ObjectId,
    pub link_id: String,
    /// Unique per install (generated client-side).
    pub install_id: String,
    /// User ID linked after signup (None until user authenticates).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    pub app_version: String,
    pub attributed_at: DateTime,
}

// ── API Request / Response Models ──

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateLinkRequest {
    /// Optional vanity slug (3-64 chars, alphanumeric + hyphens).
    #[serde(default)]
    pub custom_id: Option<String>,
    /// iOS deep link URI (e.g. "myapp://product/123").
    #[serde(default)]
    pub ios_deep_link: Option<String>,
    /// Android deep link URI.
    #[serde(default)]
    pub android_deep_link: Option<String>,
    /// Web fallback URL for desktop/unknown platforms.
    #[serde(default)]
    pub web_url: Option<String>,
    /// App Store link for iOS.
    #[serde(default)]
    pub ios_store_url: Option<String>,
    /// Play Store link for Android.
    #[serde(default)]
    pub android_store_url: Option<String>,
    /// Arbitrary key-value metadata.
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CreateLinkResponse {
    pub link_id: String,
    pub url: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LinkDetail {
    pub link_id: String,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ios_deep_link: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub android_deep_link: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub web_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ios_store_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub android_store_url: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ListLinksResponse {
    pub links: Vec<LinkDetail>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ReportAttributionRequest {
    pub link_id: String,
    pub install_id: String,
    pub app_version: String,
    /// Optional custom domain for tenant-scoped link lookup.
    #[serde(default)]
    pub domain: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AttributionResponse {
    pub success: bool,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct LinkAttributionRequest {
    pub install_id: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LinkStatsResponse {
    pub link_id: String,
    pub click_count: u64,
    pub install_count: u64,
    pub conversion_rate: f64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ResolvedLink {
    pub link_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ios_deep_link: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub android_deep_link: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub web_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ios_store_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub android_store_url: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

// ── SDK Click Models ──

#[derive(Debug, Deserialize, ToSchema)]
pub struct SdkClickRequest {
    /// Link ID to resolve.
    pub link_id: String,
    /// Optional custom domain for tenant-scoped lookup.
    #[serde(default)]
    pub domain: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SdkClickResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    pub platform: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ios_deep_link: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub android_deep_link: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub web_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ios_store_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub android_store_url: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

// ── Deferred Deep Linking Models ──

#[derive(Debug, Deserialize, ToSchema)]
pub struct DeferredLinkRequest {
    /// Token from clipboard or install referrer.
    pub token: String,
    /// Client-generated install ID for attribution.
    pub install_id: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DeferredLinkResponse {
    pub matched: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ios_deep_link: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub android_deep_link: Option<String>,
    pub metadata: Option<serde_json::Value>,
}
