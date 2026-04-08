use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredConfig {
    pub secret_key: String,
    pub base_url: String,
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
        let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
        serde_json::from_str(&text).map_err(|e| e.to_string())
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::path()?;
        let text = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(path, text).map_err(|e| e.to_string())
    }
}
