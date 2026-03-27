use rift_sdk_core::client::RiftClient;
use rift_sdk_core::error::RiftError as CoreError;
use std::sync::Once;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

uniffi::setup_scaffolding!("rift_ffi");

// ── Logging ──

static LOGGING_INIT: Once = Once::new();

fn init_logging(level: &str) {
    LOGGING_INIT.call_once(|| {
        let filter = tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(level));

        let fmt_layer = tracing_subscriber::fmt::layer()
            .with_ansi(false)
            .with_file(false)
            .with_line_number(true)
            .with_thread_ids(true)
            .with_target(true)
            .compact();

        let result = tracing_subscriber::registry()
            .with(filter)
            .with(fmt_layer)
            .try_init();

        match result {
            Ok(_) => eprintln!("[Rift SDK] Logging initialized (level: {level})"),
            Err(e) => eprintln!("[Rift SDK] Logging init skipped: {e}"),
        }
    });
}

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
    /// Log level: "trace", "debug", "info", "warn", "error". Default: "info".
    pub log_level: Option<String>,
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
        let level = config.log_level.as_deref().unwrap_or("info");
        init_logging(level);

        tracing::info!("RiftSdk initializing");
        tracing::debug!(
            has_tokio_runtime = tokio::runtime::Handle::try_current().is_ok(),
            "Tokio runtime check at construction"
        );

        Self {
            client: RiftClient::new(config.publishable_key, config.base_url),
        }
    }
}

#[uniffi::export(async_runtime = "tokio")]
impl RiftSdk {
    pub async fn click(&self, link_id: String) -> Result<ClickResult, RiftError> {
        tracing::debug!(
            has_tokio_runtime = tokio::runtime::Handle::try_current().is_ok(),
            "Tokio runtime check in click()"
        );
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
        tracing::debug!(
            has_tokio_runtime = tokio::runtime::Handle::try_current().is_ok(),
            "Tokio runtime check in report_attribution()"
        );
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
