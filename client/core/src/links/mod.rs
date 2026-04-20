use serde::{Deserialize, Serialize};

use crate::error::RiftClientError;
use crate::RiftClient;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentContext {
    pub action: Option<String>,
    pub cta: Option<String>,
    pub description: Option<String>,
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
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListLinksResponse {
    pub links: Vec<LinkDetail>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LinkStatsResponse {
    pub link_id: String,
    pub click_count: u64,
    pub install_count: u64,
    pub conversion_rate: f64,
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
}

#[derive(Debug, Serialize)]
pub struct AttributionReportRequest {
    pub link_id: String,
    pub install_id: String,
    pub app_version: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AttributionResponse {
    pub success: bool,
}

#[derive(Debug, Serialize)]
pub struct LinkAttributionRequest {
    pub install_id: String,
    pub user_id: String,
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

    pub async fn get_link_stats(
        &self,
        link_id: &str,
    ) -> Result<LinkStatsResponse, RiftClientError> {
        self.get(&format!("/v1/links/{link_id}/stats")).await
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

    pub async fn click(
        &self,
        link_id: impl Into<String>,
    ) -> Result<ClickResponse, RiftClientError> {
        self.post(
            "/v1/attribution/click",
            &ClickRequest {
                link_id: link_id.into(),
            },
            true,
        )
        .await
    }

    pub async fn report_attribution(
        &self,
        link_id: impl Into<String>,
        install_id: impl Into<String>,
        app_version: impl Into<String>,
    ) -> Result<AttributionResponse, RiftClientError> {
        self.post(
            "/v1/attribution/report",
            &AttributionReportRequest {
                link_id: link_id.into(),
                install_id: install_id.into(),
                app_version: app_version.into(),
            },
            false,
        )
        .await
    }

    /// Bind an install_id to a user_id on the server.
    ///
    /// Requires a publishable key (`pk_live_`). The endpoint is idempotent for
    /// identical `(install_id, user_id)` pairs and refuses to overwrite a
    /// previously-bound user. Used by the mobile SDK's `set_user_id` flow.
    pub async fn link_attribution(
        &self,
        install_id: impl Into<String>,
        user_id: impl Into<String>,
    ) -> Result<AttributionResponse, RiftClientError> {
        self.put(
            "/v1/attribution/link",
            &LinkAttributionRequest {
                install_id: install_id.into(),
                user_id: user_id.into(),
            },
        )
        .await
    }
}
