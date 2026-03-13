use mongodb::bson::{DateTime, Document, oid::ObjectId};
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
    /// Optional deep link destination URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination: Option<String>,
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
    /// Deep link destination URL.
    #[serde(default)]
    pub destination: Option<String>,
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
    pub destination: Option<String>,
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
    pub destination: Option<String>,
    pub metadata: Option<serde_json::Value>,
}
