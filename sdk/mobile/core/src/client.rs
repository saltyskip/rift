use crate::error::RiftError;
use crate::models::*;

const DEFAULT_BASE_URL: &str = "https://api.riftl.ink";

pub struct RiftClient {
    base_url: String,
    publishable_key: String,
    http: reqwest::Client,
}

impl RiftClient {
    pub fn new(publishable_key: String, base_url: Option<String>) -> Self {
        let base = base_url.unwrap_or_else(|| DEFAULT_BASE_URL.to_string());
        tracing::info!(base_url = %base, "RiftClient created");
        Self {
            base_url: base,
            publishable_key,
            http: reqwest::Client::new(),
        }
    }

    pub async fn click(&self, link_id: String) -> Result<ClickResponse, RiftError> {
        let url = format!("{}/v1/attribution/click", self.base_url);
        tracing::debug!(link_id = %link_id, url = %url, "Sending click request");

        let resp = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.publishable_key))
            .json(&ClickRequest { link_id })
            .send()
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Click request failed");
                e
            })?;

        tracing::debug!(status = %resp.status(), "Click response received");

        if !resp.status().is_success() {
            return Err(api_error(resp).await);
        }

        resp.json::<ClickResponse>()
            .await
            .map_err(|e| RiftError::Deserialize(e.to_string()))
    }

    pub async fn report_attribution(
        &self,
        link_id: String,
        install_id: String,
        app_version: String,
    ) -> Result<bool, RiftError> {
        let url = format!("{}/v1/attribution/report", self.base_url);
        tracing::debug!(link_id = %link_id, install_id = %install_id, url = %url, "Sending attribution report");

        let resp = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.publishable_key))
            .json(&AttributionRequest {
                link_id,
                install_id,
                app_version,
            })
            .send()
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Attribution request failed");
                e
            })?;

        tracing::debug!(status = %resp.status(), "Attribution response received");

        if !resp.status().is_success() {
            return Err(api_error(resp).await);
        }

        let result: AttributionApiResponse = resp
            .json()
            .await
            .map_err(|e| RiftError::Deserialize(e.to_string()))?;

        Ok(result.success)
    }
}

async fn api_error(resp: reqwest::Response) -> RiftError {
    let status = resp.status().as_u16();
    let body: ApiErrorBody = resp.json().await.unwrap_or(ApiErrorBody {
        error: "Unknown error".into(),
        code: None,
    });
    tracing::warn!(status = status, error = %body.error, "API error response");
    RiftError::Api {
        status,
        message: body.error,
    }
}
