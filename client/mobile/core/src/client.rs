use crate::error::RiftError;
use crate::models::*;
use rift_client_core::links::ResolvedLink;

pub struct RiftClient {
    inner: rift_client_core::RiftClient,
}

impl RiftClient {
    pub fn new(publishable_key: String, base_url: Option<String>) -> Self {
        let inner = rift_client_core::RiftClient::with_publishable_key(publishable_key, base_url);
        tracing::info!("RiftClient created");
        Self { inner }
    }

    pub async fn click(&self, link_id: String) -> Result<ClickResponse, RiftError> {
        let resp = self.inner.click(link_id).await.map_err(map_error)?;
        Ok(ClickResponse {
            link_id: resp.link_id,
            platform: resp.platform,
            ios_deep_link: resp.ios_deep_link,
            android_deep_link: resp.android_deep_link,
            web_url: resp.web_url,
            ios_store_url: resp.ios_store_url,
            android_store_url: resp.android_store_url,
            metadata: resp.metadata,
        })
    }

    pub async fn report_attribution(
        &self,
        link_id: String,
        install_id: String,
        app_version: String,
    ) -> Result<bool, RiftError> {
        let result = self
            .inner
            .report_attribution(link_id, install_id, app_version)
            .await
            .map_err(map_error)?;
        Ok(result.success)
    }

    pub async fn link_attribution(
        &self,
        install_id: String,
        user_id: String,
    ) -> Result<bool, RiftError> {
        let result = self
            .inner
            .link_attribution(install_id, user_id)
            .await
            .map_err(map_error)?;
        Ok(result.success)
    }

    pub async fn get_link(&self, link_id: String) -> Result<GetLinkResponse, RiftError> {
        let link: ResolvedLink = self.inner.resolve_link(&link_id).await.map_err(map_error)?;
        Ok(GetLinkResponse {
            link_id: link.link_id,
            ios_deep_link: link.ios_deep_link,
            android_deep_link: link.android_deep_link,
            web_url: link.web_url,
            ios_store_url: link.ios_store_url,
            android_store_url: link.android_store_url,
            metadata: link.metadata,
        })
    }
}

fn map_error(error: rift_client_core::RiftClientError) -> RiftError {
    match error {
        rift_client_core::RiftClientError::Network(message) => RiftError::Network(message),
        rift_client_core::RiftClientError::Api { status, message } => {
            RiftError::Api { status, message }
        }
        rift_client_core::RiftClientError::Deserialize(message) => RiftError::Deserialize(message),
    }
}
