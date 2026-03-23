use mongodb::bson::{oid::ObjectId, DateTime};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

// ── Database Document ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct App {
    #[serde(rename = "_id")]
    pub id: ObjectId,
    pub tenant_id: ObjectId,
    /// "ios" or "android".
    pub platform: String,
    /// iOS bundle ID (e.g. "com.example.myapp").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bundle_id: Option<String>,
    /// Apple Team ID for AASA.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub team_id: Option<String>,
    /// Android package name (e.g. "com.example.myapp").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package_name: Option<String>,
    /// Android signing certificate SHA-256 fingerprints.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha256_fingerprints: Option<Vec<String>>,
    /// App display name for landing pages.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_name: Option<String>,
    /// App icon URL for landing pages.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,
    /// Theme color (hex) for landing pages.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme_color: Option<String>,
    pub created_at: DateTime,
}

// ── API Request / Response Models ──

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateAppRequest {
    /// "ios" or "android".
    pub platform: String,
    /// iOS bundle ID.
    #[serde(default)]
    pub bundle_id: Option<String>,
    /// Apple Team ID.
    #[serde(default)]
    pub team_id: Option<String>,
    /// Android package name.
    #[serde(default)]
    pub package_name: Option<String>,
    /// Android signing certificate SHA-256 fingerprints.
    #[serde(default)]
    pub sha256_fingerprints: Option<Vec<String>>,
    /// App display name.
    #[serde(default)]
    pub app_name: Option<String>,
    /// App icon URL.
    #[serde(default)]
    pub icon_url: Option<String>,
    /// Theme color (hex).
    #[serde(default)]
    pub theme_color: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AppDetail {
    pub id: String,
    pub platform: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bundle_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub team_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha256_fingerprints: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme_color: Option<String>,
    pub created_at: String,
}
