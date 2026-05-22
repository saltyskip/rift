use serde::Deserialize;

// ── Response types (received from the API) ──

#[derive(Debug, Deserialize)]
pub struct ClickResponse {
    pub link_id: String,
    pub platform: String,
    pub ios_deep_link: Option<String>,
    pub android_deep_link: Option<String>,
    pub web_url: Option<String>,
    pub ios_store_url: Option<String>,
    pub android_store_url: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct GetLinkResponse {
    pub link_id: String,
    pub ios_deep_link: Option<String>,
    pub android_deep_link: Option<String>,
    pub web_url: Option<String>,
    pub ios_store_url: Option<String>,
    pub android_store_url: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

/// Device + app context captured by the platform wrapper from public OS
/// APIs (no permissions). Passed up to the server on `/attribute` so it
/// can distinguish `install.reinstalled` from `install.new_device` on
/// identify and enrich the lifecycle stream.
///
/// Mirror of [`rift_client_core::links::DeviceContext`] — kept as its
/// own type in the mobile core so the UniFFI binding surface doesn't
/// take a transitive dep on the cross-platform client types.
#[derive(Debug, Clone, Default)]
pub struct DeviceContext {
    pub app_version: Option<String>,
    pub platform: Option<String>,
    pub os: Option<String>,
    pub os_version: Option<String>,
    pub device_model: Option<String>,
    pub device_manufacturer: Option<String>,
    pub locale: Option<String>,
    pub region: Option<String>,
    pub timezone: Option<String>,
}

impl From<DeviceContext> for rift_client_core::links::DeviceContext {
    fn from(c: DeviceContext) -> Self {
        Self {
            app_version: c.app_version,
            platform: c.platform,
            os: c.os,
            os_version: c.os_version,
            device_model: c.device_model,
            device_manufacturer: c.device_manufacturer,
            locale: c.locale,
            region: c.region,
            timezone: c.timezone,
        }
    }
}
