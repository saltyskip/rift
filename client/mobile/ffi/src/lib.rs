use rift_sdk_core::client::RiftClient;
use rift_sdk_core::error::RiftError as CoreError;
use std::sync::{Arc, Once};
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

// ── Storage (foreign trait) ──

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum StorageError {
    #[error("Storage operation failed: {message}")]
    IoError { message: String },
}

/// Storage backend for the Rift SDK. Implemented natively on each platform:
/// - iOS: Keychain-backed (`KeychainStorage`) — persists across app reinstalls
/// - Android: SharedPreferences-backed (`SharedPrefsStorage`) — persists across launches
///
/// Rust core owns all logic (UUID generation, retry, HTTP); this trait is the
/// thin seam for platform-specific storage primitives. Methods are synchronous
/// by design — Keychain and SharedPreferences are fast in-memory lookups that
/// don't block the tokio runtime when called from async Rust code.
#[uniffi::export(with_foreign)]
pub trait RiftStorage: Send + Sync + std::fmt::Debug {
    /// Read a value by key. Returns `None` if the key is not present.
    fn get(&self, key: String) -> Result<Option<String>, StorageError>;
    /// Write a value. Overwrites any existing value for the key.
    fn set(&self, key: String, value: String) -> Result<(), StorageError>;
    /// Delete a key. Must succeed silently if the key is not present.
    fn remove(&self, key: String) -> Result<(), StorageError>;
}

fn storage_err_to_rift(e: StorageError) -> RiftError {
    let StorageError::IoError { message } = e;
    RiftError::Network {
        message: format!("storage: {message}"),
    }
}

// ── Storage keys ──

const STORAGE_KEY_INSTALL_ID: &str = "rift.install_id";
const STORAGE_KEY_USER_ID: &str = "rift.user_id";
const STORAGE_KEY_USER_ID_SYNCED: &str = "rift.user_id_synced";

// ── Config ──

#[derive(uniffi::Record)]
pub struct RiftConfig {
    pub publishable_key: String,
    pub base_url: Option<String>,
    /// Log level: "trace", "debug", "info", "warn", "error". Default: "info".
    pub log_level: Option<String>,
    /// App version string (e.g. "1.2.3"). Used by `report_attribution_for_link()`.
    /// If None, defaults to "unknown". The native convenience constructors
    /// auto-populate this from `Bundle.main` (iOS) or `PackageManager` (Android).
    pub app_version: Option<String>,
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

#[derive(uniffi::Record)]
pub struct GetLinkResult {
    pub link_id: String,
    pub ios_deep_link: Option<String>,
    pub android_deep_link: Option<String>,
    pub web_url: Option<String>,
    pub ios_store_url: Option<String>,
    pub android_store_url: Option<String>,
    pub metadata: Option<String>,
}

/// Result from `check_deferred_deep_link` — contains the link data if a
/// deferred deep link was found in the clipboard text.
#[derive(uniffi::Record)]
pub struct DeferredDeepLinkResult {
    pub link_id: String,
    pub ios_deep_link: Option<String>,
    pub android_deep_link: Option<String>,
    pub web_url: Option<String>,
    /// JSON string of arbitrary metadata, or None.
    pub metadata: Option<String>,
}

// ── SDK Object ──

#[derive(uniffi::Object)]
pub struct RiftSdk {
    client: RiftClient,
    storage: Arc<dyn RiftStorage>,
    app_version: String,
    /// Base URL for the Rift API (e.g. "https://api.riftl.ink"). Used to build
    /// the SDK conversion endpoint URL. Stored separately because `RiftClient`
    /// doesn't expose its base_url.
    api_base_url: String,
    /// Publishable key for SDK-authenticated endpoints.
    publishable_key: String,
}

#[uniffi::export]
impl RiftSdk {
    /// Construct a new SDK instance.
    ///
    /// `storage` is a platform-provided implementation of `RiftStorage` that
    /// persists `install_id`, `user_id`, and sync flags. The SDK spawns a
    /// background task on construction to retry any pending user binding
    /// left over from a previous session.
    #[uniffi::constructor]
    pub fn new(config: RiftConfig, storage: Arc<dyn RiftStorage>) -> Arc<Self> {
        let level = config.log_level.as_deref().unwrap_or("info");
        init_logging(level);

        tracing::info!("RiftSdk initializing");
        tracing::debug!(
            has_tokio_runtime = tokio::runtime::Handle::try_current().is_ok(),
            "Tokio runtime check at construction"
        );

        let api_base_url = config
            .base_url
            .clone()
            .unwrap_or_else(|| "https://api.riftl.ink".to_string());
        let publishable_key = config.publishable_key.clone();

        let sdk = Arc::new(Self {
            client: RiftClient::new(config.publishable_key, config.base_url),
            storage,
            app_version: config.app_version.unwrap_or_else(|| "unknown".to_string()),
            api_base_url,
            publishable_key,
        });

        // Retry any pending user binding from a previous session.
        //
        // Critically, we check storage synchronously *before* deciding to spawn
        // so that a fresh tenant (no stored user_id) doesn't race with a
        // subsequent `set_user_id` call. Only spawn the retry when we actually
        // find a stored user_id with `synced != "true"`.
        let needs_retry = matches!(
            (
                sdk.storage.get(STORAGE_KEY_USER_ID.to_string()).ok().flatten(),
                sdk.storage
                    .get(STORAGE_KEY_USER_ID_SYNCED.to_string())
                    .ok()
                    .flatten(),
            ),
            (Some(_), synced) if synced.as_deref() != Some("true")
        );

        if needs_retry && tokio::runtime::Handle::try_current().is_ok() {
            let sdk_bg = sdk.clone();
            tokio::spawn(async move {
                if let Err(e) = sdk_bg.retry_pending_binding().await {
                    tracing::debug!(error = ?e, "pending binding retry failed");
                }
            });
        }

        sdk
    }
}

impl RiftSdk {
    /// Internal: read-or-generate the persistent install_id.
    fn get_or_create_install_id(&self) -> Result<String, RiftError> {
        if let Some(existing) = self
            .storage
            .get(STORAGE_KEY_INSTALL_ID.to_string())
            .map_err(storage_err_to_rift)?
        {
            return Ok(existing);
        }
        let id = uuid::Uuid::new_v4().to_string();
        self.storage
            .set(STORAGE_KEY_INSTALL_ID.to_string(), id.clone())
            .map_err(storage_err_to_rift)?;
        Ok(id)
    }

    /// Internal: if there's a stored user_id that hasn't been synced to the
    /// server, attempt the binding. Called on SDK construction.
    ///
    /// Handles both `synced == Some("false")` (normal pending state) and
    /// `synced == None` (crash between writing user_id and writing the synced
    /// flag). Both mean "needs retry."
    async fn retry_pending_binding(&self) -> Result<(), RiftError> {
        let user_id = self
            .storage
            .get(STORAGE_KEY_USER_ID.to_string())
            .map_err(storage_err_to_rift)?;
        let synced = self
            .storage
            .get(STORAGE_KEY_USER_ID_SYNCED.to_string())
            .map_err(storage_err_to_rift)?;

        let Some(user_id) = user_id else {
            return Ok(());
        };
        if synced.as_deref() == Some("true") {
            return Ok(());
        }

        let install_id = self.get_or_create_install_id()?;
        self.client
            .link_attribution(install_id, user_id.clone())
            .await?;

        // Re-read user_id to guard against a race where set_user_id("usr_new")
        // overwrote storage while this background retry for "usr_old" was
        // in-flight. Only mark synced if the stored user still matches what
        // we just sent to the server.
        let current = self
            .storage
            .get(STORAGE_KEY_USER_ID.to_string())
            .map_err(storage_err_to_rift)?;
        if current.as_deref() == Some(user_id.as_str()) {
            self.storage
                .set(STORAGE_KEY_USER_ID_SYNCED.to_string(), "true".to_string())
                .map_err(storage_err_to_rift)?;
        }
        Ok(())
    }
}

#[uniffi::export]
impl RiftSdk {
    /// Return the persistent install ID, generating and storing a new UUID
    /// on first access. Stable across app launches, and (on iOS with Keychain)
    /// stable across app reinstalls.
    pub fn install_id(&self) -> Result<String, RiftError> {
        self.get_or_create_install_id()
    }

    /// Clear the bound user ID. Call on user logout. The install_id itself is
    /// preserved — only the user binding is cleared.
    pub fn clear_user_id(&self) -> Result<(), RiftError> {
        self.storage
            .remove(STORAGE_KEY_USER_ID.to_string())
            .map_err(storage_err_to_rift)?;
        self.storage
            .remove(STORAGE_KEY_USER_ID_SYNCED.to_string())
            .map_err(storage_err_to_rift)?;
        Ok(())
    }
}

#[uniffi::export(async_runtime = "tokio")]
impl RiftSdk {
    /// Fire a conversion event. Reads the bound `user_id` from storage and
    /// POSTs to the Rift API at `/v1/attribution/convert` using the publishable key.
    /// The server dedupes via `idempotency_key`.
    ///
    /// If no `user_id` is bound, logs a warning and returns (the event won't
    /// attribute without a user binding).
    pub async fn track_conversion(
        &self,
        conversion_type: String,
        idempotency_key: String,
        metadata: Option<std::collections::HashMap<String, String>>,
    ) -> Result<(), RiftError> {
        let user_id = self
            .storage
            .get(STORAGE_KEY_USER_ID.to_string())
            .map_err(storage_err_to_rift)?;
        let Some(user_id) = user_id else {
            tracing::warn!("track_conversion called but no user_id bound — call setUserId first");
            return Ok(());
        };

        let mut payload = serde_json::json!({
            "user_id": user_id,
            "type": conversion_type,
            "idempotency_key": idempotency_key,
        });
        if let Some(meta) = metadata {
            payload["metadata"] = serde_json::json!(meta);
        }

        let url = format!(
            "{}/v1/attribution/convert",
            self.api_base_url.trim_end_matches('/')
        );
        let http = reqwest::Client::new();
        match http
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.publishable_key))
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
    /// Bind the current install to a user ID. Persists locally first, then
    /// fires the server call. If the server call fails, the binding is kept
    /// as "pending" and retried on the next SDK init. Idempotent — safe to
    /// call on every app launch with the same user_id; a no-op if the user
    /// is already bound and synced.
    pub async fn set_user_id(&self, user_id: String) -> Result<(), RiftError> {
        if user_id.trim().is_empty() {
            return Err(RiftError::Api {
                status: 400,
                message: "user_id must not be empty".to_string(),
            });
        }

        // If this user_id is already stored AND already synced, no-op.
        let existing = self
            .storage
            .get(STORAGE_KEY_USER_ID.to_string())
            .map_err(storage_err_to_rift)?;
        let synced = self
            .storage
            .get(STORAGE_KEY_USER_ID_SYNCED.to_string())
            .map_err(storage_err_to_rift)?;
        if existing.as_deref() == Some(user_id.as_str()) && synced.as_deref() == Some("true") {
            return Ok(());
        }

        // Persist the new user_id and mark unsynced so retry-on-launch will
        // pick it up if the server call below fails.
        self.storage
            .set(STORAGE_KEY_USER_ID.to_string(), user_id.clone())
            .map_err(storage_err_to_rift)?;
        self.storage
            .set(STORAGE_KEY_USER_ID_SYNCED.to_string(), "false".to_string())
            .map_err(storage_err_to_rift)?;

        // Resolve install_id (generate on first call).
        let install_id = self.get_or_create_install_id()?;

        // Fire the server call. On success, mark synced. On transient failure
        // (network), leave unsynced so the next launch retries. On permanent
        // failure (404 = re-bind protection, 400 = bad request), clear the
        // unsynced state to prevent infinite retry loops.
        match self.client.link_attribution(install_id, user_id).await {
            Ok(_) => {
                self.storage
                    .set(STORAGE_KEY_USER_ID_SYNCED.to_string(), "true".to_string())
                    .map_err(storage_err_to_rift)?;
                Ok(())
            }
            Err(CoreError::Api { status, .. }) if status == 400 || status == 404 => {
                // Server rejected the binding permanently (e.g. install already
                // bound to a different user, or invalid request). Clear the
                // pending state so we don't retry on every launch.
                tracing::warn!(
                    status,
                    "link_attribution permanently rejected; clearing pending state"
                );
                self.storage
                    .remove(STORAGE_KEY_USER_ID.to_string())
                    .map_err(storage_err_to_rift)?;
                self.storage
                    .remove(STORAGE_KEY_USER_ID_SYNCED.to_string())
                    .map_err(storage_err_to_rift)?;
                Err(RiftError::Api {
                    status,
                    message: "User binding rejected by server".to_string(),
                })
            }
            Err(e) => {
                tracing::warn!(error = ?e, "link_attribution failed; will retry on next launch");
                Err(e.into())
            }
        }
    }

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

    pub async fn get_link(&self, link_id: String) -> Result<GetLinkResult, RiftError> {
        let resp = self.client.get_link(link_id).await?;
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

    /// Simplified attribution reporting — uses the SDK's internal install_id
    /// and the `app_version` from config. One argument instead of three.
    pub async fn report_attribution_for_link(&self, link_id: String) -> Result<bool, RiftError> {
        let install_id = self.get_or_create_install_id()?;
        Ok(self
            .client
            .report_attribution(link_id, install_id, self.app_version.clone())
            .await?)
    }

    /// One-call deferred deep linking. Parses the clipboard text for a Rift
    /// link, reports attribution if found, and returns the link data for
    /// navigation. Returns `None` if no Rift link is found.
    ///
    /// The caller must read the clipboard and pass the text in — the SDK
    /// deliberately does NOT read the clipboard itself because iOS 16+
    /// shows a paste permission dialog, and the app should control when
    /// that dialog appears.
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

        // Report attribution (fire-and-forget on failure — don't block navigation).
        if let Err(e) = self.report_attribution_for_link(link_id.clone()).await {
            tracing::warn!(error = ?e, "deferred deep link attribution failed");
        }

        // Fetch link data for navigation.
        match self.client.get_link(link_id.clone()).await {
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

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
