use serde::{Deserialize, Serialize};

use crate::error::RiftClientError;
use crate::RiftClient;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentContext {
    pub action: Option<String>,
    pub cta: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialPreview {
    pub title: Option<String>,
    pub description: Option<String>,
    pub image_url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CreateLinkRequest {
    pub custom_id: Option<String>,
    pub ios_deep_link: Option<String>,
    pub android_deep_link: Option<String>,
    pub web_url: Option<String>,
    pub ios_store_url: Option<String>,
    pub android_store_url: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub agent_context: Option<AgentContext>,
    pub social_preview: Option<SocialPreview>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateLinkResponse {
    pub link_id: String,
    pub url: String,
    pub expires_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LinkDetail {
    pub link_id: String,
    pub url: String,
    pub ios_deep_link: Option<String>,
    pub android_deep_link: Option<String>,
    pub web_url: Option<String>,
    pub ios_store_url: Option<String>,
    pub android_store_url: Option<String>,
    pub created_at: String,
    pub agent_context: Option<AgentContext>,
    pub social_preview: Option<SocialPreview>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListLinksResponse {
    pub links: Vec<LinkDetail>,
    pub next_cursor: Option<String>,
}

// ── /v1/analytics/stats response shapes ──
//
// Mirrors `services/analytics/models::FunnelResult` on the server. SDK owns
// its own types per CLAUDE.md's mobile-SDK convention; the shape doesn't
// drift in practice because `cargo test` would break the moment server JSON
// stops deserializing here.

#[derive(Debug, Serialize, Deserialize)]
pub struct FunnelStats {
    pub from: String,
    pub to: String,
    pub link_ids: Vec<String>,
    pub credit: String,
    pub funnel: Funnel,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Funnel {
    pub clicks: u64,
    pub new_users: FunnelNewUsers,
    pub returning_users: FunnelReturningUsers,
    /// Conversion event counts keyed by type ("signup", "purchase", etc.).
    pub conversions: std::collections::BTreeMap<String, u64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FunnelNewUsers {
    pub installed: u64,
    pub identified: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FunnelReturningUsers {
    pub reinstalled: u64,
    pub new_device: u64,
    pub engaged: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TimeseriesDataPoint {
    pub date: String,
    pub click_count: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TimeseriesResponse {
    pub link_id: String,
    pub granularity: String,
    pub from: String,
    pub to: String,
    pub data: Vec<TimeseriesDataPoint>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResolvedLink {
    pub link_id: String,
    pub ios_deep_link: Option<String>,
    pub android_deep_link: Option<String>,
    pub web_url: Option<String>,
    pub ios_store_url: Option<String>,
    pub android_store_url: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub agent_context: Option<AgentContext>,
    pub social_preview: Option<SocialPreview>,
}

#[derive(Debug, Serialize)]
pub struct ClickRequest {
    pub link_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClickResponse {
    pub link_id: String,
    pub platform: String,
    pub ios_deep_link: Option<String>,
    pub android_deep_link: Option<String>,
    pub web_url: Option<String>,
    pub ios_store_url: Option<String>,
    pub android_store_url: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub agent_context: Option<AgentContext>,
    pub social_preview: Option<SocialPreview>,
}

#[derive(Debug, Clone, Default)]
pub struct QrCodeOptions {
    pub logo: Option<String>,
    pub size: Option<u32>,
    pub level: Option<String>,
    pub fg_color: Option<String>,
    pub bg_color: Option<String>,
    pub hide_logo: bool,
    pub margin: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct AttributeRequest {
    pub link_id: String,
    pub install_id: String,
    pub app_version: String,
    /// Device + app context captured from public OS APIs. Sent so the
    /// server can enrich `install.created` events and distinguish
    /// `install.reinstalled` from `install.new_device` on identify.
    /// Absent on older callers; server treats missing as "no context."
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<DeviceContext>,
}

/// Device / app metadata captured from public OS APIs (no permissions).
/// Mirrors `crate::services::links::models::AttributeContext` on the
/// server side. All fields optional — callers populate what they can
/// from their platform's `UIDevice` / `Build` equivalent.
#[derive(Debug, Clone, Default, Serialize)]
pub struct DeviceContext {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub os: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub os_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_manufacturer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locale: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timezone: Option<String>,
}

/// Response from `POST /v1/lifecycle/attribute`. See server-side
/// `AttributeResponse` for forward-compat fields landing here over time.
#[derive(Debug, Serialize, Deserialize)]
pub struct AttributeResponse {
    pub success: bool,
}

#[derive(Debug, Serialize)]
pub struct IdentifyRequest {
    pub install_id: String,
    pub user_id: String,
}

/// Response from `PUT /v1/lifecycle/identify`. See server-side
/// `IdentifyResponse` for forward-compat fields landing here over time.
#[derive(Debug, Serialize, Deserialize)]
pub struct IdentifyResponse {
    pub success: bool,
}

impl RiftClient {
    pub async fn create_link(
        &self,
        request: &CreateLinkRequest,
    ) -> Result<CreateLinkResponse, RiftClientError> {
        self.post("/v1/links", request, false).await
    }

    pub async fn get_link(&self, link_id: &str) -> Result<LinkDetail, RiftClientError> {
        self.get(&format!("/v1/links/{link_id}")).await
    }

    pub async fn list_links(
        &self,
        limit: Option<i64>,
        cursor: Option<&str>,
    ) -> Result<ListLinksResponse, RiftClientError> {
        let mut path = String::from("/v1/links");
        let mut parts = Vec::new();
        if let Some(limit) = limit {
            parts.push(format!("limit={limit}"));
        }
        if let Some(cursor) = cursor {
            parts.push(format!("cursor={cursor}"));
        }
        if !parts.is_empty() {
            path.push('?');
            path.push_str(&parts.join("&"));
        }
        self.get(&path).await
    }

    pub async fn get_link_timeseries(
        &self,
        link_id: &str,
        from: &str,
        to: &str,
    ) -> Result<TimeseriesResponse, RiftClientError> {
        self.get(&format!(
            "/v1/links/{link_id}/timeseries?from={}&to={}&granularity=daily",
            urlencoding::encode(from),
            urlencoding::encode(to)
        ))
        .await
    }

    pub async fn resolve_link(&self, link_id: &str) -> Result<ResolvedLink, RiftClientError> {
        self.get(&format!("/r/{link_id}")).await
    }

    /// Funnel stats across one or more links. Maps to
    /// `GET /v1/analytics/stats?link_ids=…&from=…&to=…&credit=…`.
    ///
    /// `from`/`to` are RFC 3339 strings — server defaults to the last 30
    /// days when either is omitted. `credit` is `last_touch` (default),
    /// `first_touch`, or `touched`.
    ///
    /// Named `get_funnel_stats` (not `get_link_stats`) to disambiguate
    /// from the per-link counts on `/v1/links/{id}/stats`.
    pub async fn get_funnel_stats(
        &self,
        link_ids: &[String],
        from: Option<&str>,
        to: Option<&str>,
        credit: Option<&str>,
    ) -> Result<FunnelStats, RiftClientError> {
        let mut query: Vec<(&str, String)> = vec![("link_ids", link_ids.join(","))];
        if let Some(v) = from {
            query.push(("from", v.to_string()));
        }
        if let Some(v) = to {
            query.push(("to", v.to_string()));
        }
        if let Some(v) = credit {
            query.push(("credit", v.to_string()));
        }
        self.get_with_query("/v1/analytics/stats", &query).await
    }

    pub async fn get_link_qr_png(
        &self,
        link_id: &str,
        options: &QrCodeOptions,
    ) -> Result<Vec<u8>, RiftClientError> {
        self.get_bytes(&qr_path(link_id, "png", options)).await
    }

    pub async fn get_link_qr_svg(
        &self,
        link_id: &str,
        options: &QrCodeOptions,
    ) -> Result<Vec<u8>, RiftClientError> {
        self.get_bytes(&qr_path(link_id, "svg", options)).await
    }

    pub async fn click(
        &self,
        link_id: impl Into<String>,
    ) -> Result<ClickResponse, RiftClientError> {
        self.post(
            "/v1/lifecycle/click",
            &ClickRequest {
                link_id: link_id.into(),
            },
            true,
        )
        .await
    }

    /// Record an attribution event — install (or already-installed user)
    /// touched a link. Always appends to the server's event log;
    /// re-attribution of an existing install is supported and intended.
    /// Requires a publishable key (`pk_live_`).
    pub async fn attribute(
        &self,
        link_id: impl Into<String>,
        install_id: impl Into<String>,
        app_version: impl Into<String>,
    ) -> Result<AttributeResponse, RiftClientError> {
        self.attribute_with_context(link_id, install_id, app_version, None)
            .await
    }

    /// Same as [`Self::attribute`] but with an optional device-context
    /// payload. Platform SDKs (iOS / Android) read their UIDevice / Build
    /// equivalents and pass the captured values up so the server can
    /// distinguish reinstall vs new_device on identify.
    pub async fn attribute_with_context(
        &self,
        link_id: impl Into<String>,
        install_id: impl Into<String>,
        app_version: impl Into<String>,
        context: Option<DeviceContext>,
    ) -> Result<AttributeResponse, RiftClientError> {
        self.post(
            "/v1/lifecycle/attribute",
            &AttributeRequest {
                link_id: link_id.into(),
                install_id: install_id.into(),
                app_version: app_version.into(),
                context,
            },
            false,
        )
        .await
    }

    /// Bind an install_id to a user_id on the server.
    ///
    /// Requires a publishable key (`pk_live_`). Idempotent for identical
    /// `(install_id, user_id)` pairs and refuses to overwrite a
    /// previously-bound user. Used by the mobile SDK's `set_user_id` flow.
    pub async fn identify(
        &self,
        install_id: impl Into<String>,
        user_id: impl Into<String>,
    ) -> Result<IdentifyResponse, RiftClientError> {
        self.put(
            "/v1/lifecycle/identify",
            &IdentifyRequest {
                install_id: install_id.into(),
                user_id: user_id.into(),
            },
        )
        .await
    }
}

fn qr_path(link_id: &str, format: &str, options: &QrCodeOptions) -> String {
    let mut path = format!("/v1/links/{link_id}/qr.{format}");
    let mut parts = Vec::new();
    if let Some(logo) = &options.logo {
        parts.push(format!("logo={}", urlencoding::encode(logo)));
    }
    if let Some(size) = options.size {
        parts.push(format!("size={size}"));
    }
    if let Some(level) = &options.level {
        parts.push(format!("level={}", urlencoding::encode(level)));
    }
    if let Some(fg_color) = &options.fg_color {
        parts.push(format!("fgColor={}", urlencoding::encode(fg_color)));
    }
    if let Some(bg_color) = &options.bg_color {
        parts.push(format!("bgColor={}", urlencoding::encode(bg_color)));
    }
    if options.hide_logo {
        parts.push("hideLogo=true".to_string());
    }
    if let Some(margin) = options.margin {
        parts.push(format!("margin={margin}"));
    }
    if !parts.is_empty() {
        path.push('?');
        path.push_str(&parts.join("&"));
    }
    path
}
