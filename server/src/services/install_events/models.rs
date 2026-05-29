//! Data types for `services/install_events/` — the install lifecycle stream.
//!
//! Server-derived events that complement `attribution_events` (link touches)
//! with `install_events` (install lifecycle): created, opened, identified,
//! reinstalled, new_device. The SDK never emits these directly — they are
//! fanned out server-side from `/lifecycle/attribute` and
//! `/lifecycle/identify` calls.
//!
//! Stored in a regular (non-time-series) MongoDB collection because the
//! collection is low-volume and identity-shaped — point lookups by
//! install_id are the hot path, not time-range scans.

use mongodb::bson::DateTime;

use crate::core::public_id::{InstallEventId, TenantId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum InstallEventType {
    /// First time the server has seen this install_id.
    Created,
    /// Known install pinged the server again (returning user activity).
    Opened,
    /// Install was bound to a user via /lifecycle/identify.
    Identified,
    /// Known user appeared on a new install_id with the SAME device_model
    /// as a prior install (uninstall-then-reinstall, Android-detectable).
    Reinstalled,
    /// Known user appeared on a new install_id with a DIFFERENT
    /// device_model from all prior installs (multi-device expansion).
    NewDevice,
}

/// Device / app context captured by the SDK on `/attribute`. All fields
/// are optional because the SDK may pre-date the context-capture release
/// (defaulting to `None` until the SDK is upgraded).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InstallContext {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub os: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub os_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_manufacturer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locale: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timezone: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallEvent {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<InstallEventId>,
    pub tenant_id: TenantId,
    pub install_id: String,
    pub event_type: InstallEventType,
    pub timestamp: DateTime,

    // ── Type-specific fields (all optional; populated by event type) ──
    /// Populated on install.identified, install.reinstalled,
    /// install.new_device.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,

    /// Populated on install.created (snapshot at first sighting).
    #[serde(default, flatten)]
    pub context: InstallContext,

    /// Populated on install.reinstalled and install.new_device.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prior_install_ids: Option<Vec<String>>,
}
