use rift_sdk_core::client::RiftClient;
use rift_sdk_core::error::RiftError as CoreError;
use std::sync::LazyLock;
use tokio::runtime::Runtime;

uniffi::setup_scaffolding!("rift_ffi");

// ── Global Runtime ──

static RUNTIME: LazyLock<Runtime> = LazyLock::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("Failed to create Tokio runtime")
});

// ── Error ──

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum RiftError {
    #[error("Network error: {message}")]
    Network { message: String },

    #[error("API error ({status}): {message}")]
    Api { status: u16, message: String },

    #[error("Deserialization error: {message}")]
    Deserialize { message: String },
}

impl From<CoreError> for RiftError {
    fn from(e: CoreError) -> Self {
        match e {
            CoreError::Network(msg) => RiftError::Network { message: msg },
            CoreError::Api { status, message } => RiftError::Api { status, message },
            CoreError::Deserialize(msg) => RiftError::Deserialize { message: msg },
        }
    }
}

// ── Config ──

#[derive(uniffi::Record)]
pub struct RiftConfig {
    pub publishable_key: String,
    pub base_url: Option<String>,
}

// ── Response Records ──

#[derive(uniffi::Record)]
pub struct ClickResult {
    pub link_id: String,
    pub platform: String,
    pub ios_deep_link: Option<String>,
    pub android_deep_link: Option<String>,
    pub web_url: Option<String>,
    pub ios_store_url: Option<String>,
    pub android_store_url: Option<String>,
    /// JSON string of arbitrary metadata, or None.
    pub metadata: Option<String>,
}

// ── SDK Object ──

#[derive(uniffi::Object)]
pub struct RiftSdk {
    client: RiftClient,
}

#[uniffi::export]
impl RiftSdk {
    #[uniffi::constructor]
    pub fn new(config: RiftConfig) -> Self {
        // Touch the runtime so it's initialized before any async calls.
        let _ = &*RUNTIME;
        Self {
            client: RiftClient::new(config.publishable_key, config.base_url),
        }
    }
}

#[uniffi::export(async_runtime = "tokio")]
impl RiftSdk {
    pub async fn click(&self, link_id: String) -> Result<ClickResult, RiftError> {
        let resp = self.client.click(link_id).await?;
        Ok(ClickResult {
            link_id: resp.link_id,
            platform: resp.platform,
            ios_deep_link: resp.ios_deep_link,
            android_deep_link: resp.android_deep_link,
            web_url: resp.web_url,
            ios_store_url: resp.ios_store_url,
            android_store_url: resp.android_store_url,
            metadata: resp.metadata.map(|v| v.to_string()),
        })
    }

    pub async fn report_attribution(
        &self,
        link_id: String,
        install_id: String,
        app_version: String,
    ) -> Result<bool, RiftError> {
        Ok(self
            .client
            .report_attribution(link_id, install_id, app_version)
            .await?)
    }
}

// ── Free functions ──

#[uniffi::export]
pub fn parse_clipboard_link(text: String) -> Option<String> {
    rift_sdk_core::parser::parse_clipboard_link(&text)
}

#[uniffi::export]
pub fn parse_referrer_link(referrer: String) -> Option<String> {
    rift_sdk_core::parser::parse_referrer_link(&referrer)
}
