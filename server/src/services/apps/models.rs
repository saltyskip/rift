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
    #[schema(example = "ios")]
    pub platform: String,
    /// iOS bundle ID.
    #[serde(default)]
    #[schema(example = "com.tablefour.app")]
    pub bundle_id: Option<String>,
    /// Apple Team ID.
    #[serde(default)]
    #[schema(example = "A1B2C3D4E5")]
    pub team_id: Option<String>,
    /// Android package name.
    #[serde(default)]
    #[schema(example = "com.tablefour.app")]
    pub package_name: Option<String>,
    /// Android signing certificate SHA-256 fingerprints.
    #[serde(default)]
    pub sha256_fingerprints: Option<Vec<String>>,
    /// App display name.
    #[serde(default)]
    #[schema(example = "TableFour")]
    pub app_name: Option<String>,
    /// App icon URL.
    #[serde(default)]
    #[schema(example = "https://cdn.tablefour.com/icon-512.png")]
    pub icon_url: Option<String>,
    /// Theme color (hex).
    #[serde(default)]
    #[schema(example = "#FF6B35")]
    pub theme_color: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AppDetail {
    #[schema(example = "665a1b2c3d4e5f6a7b8c9d0e")]
    pub id: String,
    #[schema(example = "ios")]
    pub platform: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "com.tablefour.app")]
    pub bundle_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "A1B2C3D4E5")]
    pub team_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "com.tablefour.app")]
    pub package_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha256_fingerprints: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "TableFour")]
    pub app_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "https://cdn.tablefour.com/icon-512.png")]
    pub icon_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "#FF6B35")]
    pub theme_color: Option<String>,
    #[schema(example = "2025-06-15T10:30:00Z")]
    pub created_at: String,
}
