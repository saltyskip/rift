use crate::error::RiftError;
use crate::models::*;

const DEFAULT_BASE_URL: &str = "https://api.riftl.ink";

pub struct RiftClient {
    base_url: String,
    http: reqwest::Client,
}

impl RiftClient {
    pub fn new(base_url: Option<String>) -> Self {
        Self {
            base_url: base_url.unwrap_or_else(|| DEFAULT_BASE_URL.to_string()),
            http: reqwest::Client::new(),
        }
    }

    pub async fn click(
        &self,
        link_id: String,
        domain: Option<String>,
    ) -> Result<ClickResponse, RiftError> {
        let url = format!("{}/v1/sdk/click", self.base_url);
        let resp = self
            .http
            .post(&url)
            .json(&ClickRequest { link_id, domain })
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(api_error(resp).await);
        }

        resp.json::<ClickResponse>()
            .await
            .map_err(|e| RiftError::Deserialize(e.to_string()))
    }

    pub async fn resolve_deferred(
        &self,
        link_id: String,
        install_id: String,
        domain: Option<String>,
    ) -> Result<DeferredResponse, RiftError> {
        let url = format!("{}/v1/deferred", self.base_url);
        let resp = self
            .http
            .post(&url)
            .json(&DeferredRequest {
                link_id,
                install_id,
                domain,
            })
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(api_error(resp).await);
        }

        resp.json::<DeferredResponse>()
            .await
            .map_err(|e| RiftError::Deserialize(e.to_string()))
    }

    pub async fn report_attribution(
        &self,
        link_id: String,
        install_id: String,
        app_version: String,
        domain: Option<String>,
    ) -> Result<bool, RiftError> {
        let url = format!("{}/v1/attribution", self.base_url);
        let resp = self
            .http
            .post(&url)
            .json(&AttributionRequest {
                link_id,
                install_id,
                app_version,
                domain,
            })
            .send()
            .await?;

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
    RiftError::Api {
        status,
        message: body.error,
    }
}
