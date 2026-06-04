use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// On-disk CLI credentials. Exactly one of `secret_key` / `session_token` is
/// set: `rift login --api-key` (or `rift init`) writes `secret_key`; the
/// browser `rift login` writes `session_token`. Both are `Option` and
/// `skip_serializing_if` so existing `{ "secret_key", "base_url" }` configs
/// keep deserializing unchanged.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StoredConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secret_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_token: Option<String>,
    pub base_url: String,
}

impl StoredConfig {
    /// Build a config holding a secret key.
    pub fn from_secret_key(secret_key: String, base_url: String) -> Self {
        Self {
            secret_key: Some(secret_key),
            session_token: None,
            base_url,
        }
    }

    /// Build a config holding a browser-login session token.
    pub fn from_session_token(session_token: String, base_url: String) -> Self {
        Self {
            secret_key: None,
            session_token: Some(session_token),
            base_url,
        }
    }
}

impl StoredConfig {
    pub fn path() -> Result<PathBuf, String> {
        let mut path =
            dirs::config_dir().ok_or_else(|| "Could not resolve config dir".to_string())?;
        path.push("rift");
        fs::create_dir_all(&path).map_err(|e| e.to_string())?;
        path.push("config.json");
        Ok(path)
    }

    pub fn load() -> Result<Self, String> {
        let path = Self::path()?;
        Self::load_from(&path)
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::path()?;
        self.save_to(&path)
    }

    pub fn load_from(path: &Path) -> Result<Self, String> {
        let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
        serde_json::from_str(&text).map_err(|e| e.to_string())
    }

    pub fn save_to(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let text = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(path, text).map_err(|e| e.to_string())
    }
}
