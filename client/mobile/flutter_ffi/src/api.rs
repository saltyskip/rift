use flutter_rust_bridge::frb;
use rift_sdk_core::client::RiftClient;
use rift_sdk_core::error::RiftError as CoreError;
use std::sync::{Arc, Mutex, Once};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

// ── Init ──

static LOGGING_INIT: Once = Once::new();

#[frb(init)]
pub fn init_app() {
    // flutter_rust_bridge manages the async runtime internally via the generated
    // bridge code. User initialization (logging, etc.) happens in RiftSdk::create().
}

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
            Ok(_) => eprintln!("[Rift Flutter SDK] Logging initialized (level: {level})"),
            Err(e) => eprintln!("[Rift Flutter SDK] Logging init skipped: {e}"),
        }
    });
}

// ── Error ──

#[derive(Debug, thiserror::Error)]
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

// ── Config + State ──

pub struct RiftConfig {
    pub publishable_key: String,
    pub base_url: Option<String>,
    /// "trace" | "debug" | "info" | "warn" | "error". Default: "info".
    pub log_level: Option<String>,
    /// App version string (e.g. "1.2.3"). Defaults to "unknown".
    pub app_version: Option<String>,
}

/// Snapshot of SDK state that the Dart layer must persist across launches.
/// The Dart wrapper loads this from storage before calling `RiftSdk.create`
/// and writes it back whenever a state-mutating method returns a new snapshot.
pub struct RiftState {
    pub install_id: String,
    pub user_id: Option<String>,
    pub user_id_synced: bool,
}

// ── Response types ──

pub struct ClickResult {
    pub link_id: String,
    pub platform: String,
    pub ios_deep_link: Option<String>,
    pub android_deep_link: Option<String>,
    pub web_url: Option<String>,
    pub ios_store_url: Option<String>,
    pub android_store_url: Option<String>,
    /// JSON string of arbitrary link metadata, or None.
    pub metadata: Option<String>,
}

pub struct GetLinkResult {
    pub link_id: String,
    pub ios_deep_link: Option<String>,
    pub android_deep_link: Option<String>,
    pub web_url: Option<String>,
    pub ios_store_url: Option<String>,
    pub android_store_url: Option<String>,
    pub metadata: Option<String>,
}

pub struct DeferredDeepLinkResult {
    pub link_id: String,
    pub ios_deep_link: Option<String>,
    pub android_deep_link: Option<String>,
    pub web_url: Option<String>,
    pub metadata: Option<String>,
}

// ── SDK ──

struct Inner {
    client: RiftClient,
    install_id: Mutex<String>,
    user_id: Mutex<Option<String>>,
    user_id_synced: Mutex<bool>,
    api_base_url: String,
    publishable_key: String,
    app_version: String,
}

/// Main Rift SDK object. Obtain via `RiftSdk.create(...)`.
///
/// This crate uses flutter_rust_bridge for Dart/Flutter bindings.
/// Storage is owned by the Dart layer: load `RiftState` from your preferred
/// storage backend before calling `create`, and persist the returned `RiftState`
/// after any call that mutates state (`setUserId`, `clearUserId`).
#[frb(opaque)]
pub struct RiftSdk {
    inner: Arc<Inner>,
}

impl RiftSdk {
    /// Construct a new SDK instance. Load persisted state from Dart storage and
    /// pass it here; if `None` (first launch), a new install_id is generated.
    /// Check `getState().install_id` after construction and persist if it changed.
    pub async fn create(config: RiftConfig, state: Option<RiftState>) -> RiftSdk {
        let level = config.log_level.as_deref().unwrap_or("info");
        init_logging(level);

        let install_id = state
            .as_ref()
            .map(|s| s.install_id.clone())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        let user_id = state.as_ref().and_then(|s| s.user_id.clone());
        let user_id_synced = state.as_ref().map(|s| s.user_id_synced).unwrap_or(false);

        let api_base_url = config
            .base_url
            .clone()
            .unwrap_or_else(|| "https://api.riftl.ink".to_string());
        let publishable_key = config.publishable_key.clone();

        let inner = Arc::new(Inner {
            client: RiftClient::new(config.publishable_key, config.base_url),
            install_id: Mutex::new(install_id),
            user_id: Mutex::new(user_id),
            user_id_synced: Mutex::new(user_id_synced),
            api_base_url,
            publishable_key,
            app_version: config.app_version.unwrap_or_else(|| "unknown".to_string()),
        });

        // Retry pending user binding from a previous session.
        let needs_retry = matches!(
            (
                inner.user_id.lock().unwrap().clone(),
                *inner.user_id_synced.lock().unwrap(),
            ),
            (Some(_), false)
        );
        if needs_retry {
            let inner_bg = Arc::clone(&inner);
            tokio::spawn(async move {
                if let Err(e) = retry_pending_binding(&inner_bg).await {
                    tracing::debug!(error = ?e, "pending binding retry failed");
                }
            });
        }

        RiftSdk { inner }
    }

    /// Current SDK state snapshot. Persist this after construction to save a
    /// newly-generated install_id, and after any state-mutating call.
    #[frb(sync)]
    pub fn get_state(&self) -> RiftState {
        snapshot(&self.inner)
    }

    /// The persistent install ID. Stable across launches; on iOS (with
    /// KeychainStorage) stable across reinstalls.
    #[frb(sync)]
    pub fn get_install_id(&self) -> String {
        self.inner.install_id.lock().unwrap().clone()
    }

    /// Bind a user ID to this install. Persists locally and calls the server.
    /// On network failure the binding is left as unsynced and retried on the
    /// next `create` call. Returns the new state to persist.
    pub async fn set_user_id(&self, user_id: String) -> Result<RiftState, RiftError> {
        if user_id.trim().is_empty() {
            return Err(RiftError::Api {
                status: 400,
                message: "user_id must not be empty".to_string(),
            });
        }

        let existing = self.inner.user_id.lock().unwrap().clone();
        let synced = *self.inner.user_id_synced.lock().unwrap();
        if existing.as_deref() == Some(user_id.as_str()) && synced {
            return Ok(snapshot(&self.inner));
        }

        *self.inner.user_id.lock().unwrap() = Some(user_id.clone());
        *self.inner.user_id_synced.lock().unwrap() = false;

        let install_id = self.inner.install_id.lock().unwrap().clone();
        match self
            .inner
            .client
            .identify(install_id, user_id.clone())
            .await
        {
            Ok(_) => {
                *self.inner.user_id_synced.lock().unwrap() = true;
                Ok(snapshot(&self.inner))
            }
            Err(CoreError::Api { status, .. }) if status == 400 || status == 404 => {
                tracing::warn!(
                    status,
                    "identify permanently rejected; clearing pending state"
                );
                *self.inner.user_id.lock().unwrap() = None;
                *self.inner.user_id_synced.lock().unwrap() = false;
                Err(RiftError::Api {
                    status,
                    message: "User binding rejected by server".to_string(),
                })
            }
            Err(e) => {
                tracing::warn!(error = ?e, "identify failed; will retry on next launch");
                Err(e.into())
            }
        }
    }

    /// Clear the bound user ID (call on logout). Returns the new state to persist.
    #[frb(sync)]
    pub fn clear_user_id(&self) -> RiftState {
        *self.inner.user_id.lock().unwrap() = None;
        *self.inner.user_id_synced.lock().unwrap() = false;
        snapshot(&self.inner)
    }

    /// Resolve a link and return routing destinations.
    pub async fn click(&self, link_id: String) -> Result<ClickResult, RiftError> {
        let resp = self.inner.client.click(link_id).await?;
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

    /// Report attribution for this install. Returns `true` on success.
    pub async fn attribute_link(&self, link_id: String) -> Result<bool, RiftError> {
        let install_id = self.inner.install_id.lock().unwrap().clone();
        let app_version = self.inner.app_version.clone();
        Ok(self
            .inner
            .client
            .attribute(link_id, install_id, app_version)
            .await?)
    }

    /// Fetch link routing destinations without recording a click.
    pub async fn get_link(&self, link_id: String) -> Result<GetLinkResult, RiftError> {
        let resp = self.inner.client.get_link(link_id).await?;
        Ok(GetLinkResult {
            link_id: resp.link_id,
            ios_deep_link: resp.ios_deep_link,
            android_deep_link: resp.android_deep_link,
            web_url: resp.web_url,
            ios_store_url: resp.ios_store_url,
            android_store_url: resp.android_store_url,
            metadata: resp.metadata.map(|v| v.to_string()),
        })
    }

    /// Parse clipboard text for a Rift link, report attribution if found, and
    /// return link data for navigation. Pass `None` if clipboard is empty.
    ///
    /// The caller must read the clipboard themselves — the SDK does not request
    /// clipboard permission directly.
    pub async fn check_deferred_deep_link(
        &self,
        clipboard_text: Option<String>,
    ) -> Result<Option<DeferredDeepLinkResult>, RiftError> {
        let Some(text) = clipboard_text else {
            return Ok(None);
        };
        let Some(link_id) = rift_sdk_core::parser::parse_clipboard_link(&text) else {
            return Ok(None);
        };

        if let Err(e) = self.attribute_link(link_id.clone()).await {
            tracing::warn!(error = ?e, "deferred deep link attribution failed");
        }

        match self.inner.client.get_link(link_id.clone()).await {
            Ok(resp) => Ok(Some(DeferredDeepLinkResult {
                link_id,
                ios_deep_link: resp.ios_deep_link,
                android_deep_link: resp.android_deep_link,
                web_url: resp.web_url,
                metadata: resp.metadata.map(|v| v.to_string()),
            })),
            Err(e) => {
                tracing::warn!(error = ?e, "deferred deep link fetch failed");
                Err(e.into())
            }
        }
    }

    /// Fire a conversion event. No-op (with a warning) if no user_id is bound.
    pub async fn track_conversion(
        &self,
        conversion_type: String,
        idempotency_key: String,
        metadata: Option<String>,
    ) -> Result<(), RiftError> {
        let user_id = self.inner.user_id.lock().unwrap().clone();
        let Some(user_id) = user_id else {
            tracing::warn!("track_conversion called but no user_id bound — call setUserId first");
            return Ok(());
        };

        let mut payload = serde_json::json!({
            "user_id": user_id,
            "type": conversion_type,
            "idempotency_key": idempotency_key,
        });
        if let Some(meta_str) = metadata {
            if let Ok(meta_val) = serde_json::from_str::<serde_json::Value>(&meta_str) {
                payload["metadata"] = meta_val;
            }
        }

        let url = format!(
            "{}/v1/lifecycle/convert",
            self.inner.api_base_url.trim_end_matches('/')
        );
        let http = reqwest::Client::new();
        match http
            .post(&url)
            .header(
                "Authorization",
                format!("Bearer {}", self.inner.publishable_key),
            )
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
        {
            Ok(r) => tracing::debug!(status = %r.status(), "conversion event sent"),
            Err(e) => tracing::warn!(error = %e, "conversion event failed"),
        }

        Ok(())
    }
}

// ── Free functions ──

/// Extract a Rift link_id from a clipboard string (URL or "rift:<id>" format).
pub fn parse_clipboard_link(text: String) -> Option<String> {
    rift_sdk_core::parser::parse_clipboard_link(&text)
}

/// Extract a Rift link_id from an Android install referrer query string.
pub fn parse_referrer_link(referrer: String) -> Option<String> {
    rift_sdk_core::parser::parse_referrer_link(&referrer)
}

// ── Helpers ──

fn snapshot(inner: &Inner) -> RiftState {
    RiftState {
        install_id: inner.install_id.lock().unwrap().clone(),
        user_id: inner.user_id.lock().unwrap().clone(),
        user_id_synced: *inner.user_id_synced.lock().unwrap(),
    }
}

async fn retry_pending_binding(inner: &Inner) -> Result<(), RiftError> {
    let user_id = inner.user_id.lock().unwrap().clone();
    let synced = *inner.user_id_synced.lock().unwrap();

    let Some(user_id) = user_id else {
        return Ok(());
    };
    if synced {
        return Ok(());
    }

    let install_id = inner.install_id.lock().unwrap().clone();
    inner.client.identify(install_id, user_id.clone()).await?;

    // Only mark synced if the stored user_id is still the same (guard against
    // a concurrent set_user_id call).
    let current = inner.user_id.lock().unwrap().clone();
    if current.as_deref() == Some(user_id.as_str()) {
        *inner.user_id_synced.lock().unwrap() = true;
    }

    Ok(())
}
