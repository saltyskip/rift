//! Data types for `services/app_users/` — the end-user identity record.
//!
//! An `AppUserDoc` represents one identified end-user of a tenant's app.
//! Each row is keyed by `(tenant_id, user_id)`. The `install_ids` array
//! accumulates every install the user has ever been bound to, supporting
//! multi-device and reinstall scenarios via a single reverse-lookup index.
//!
//! This is *not* `services/auth/users/` (Rift team members). Those are two
//! distinct concepts — Rift's customers are tenants, their team members are
//! `users`, and the end-users of the customer's app are `app_users`.

use mongodb::bson::{oid::ObjectId, DateTime};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppUserDoc {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub tenant_id: ObjectId,
    /// Customer-supplied identifier for the end-user. Unique within tenant.
    pub user_id: String,
    /// Every install_id ever bound to this user. Accumulates over time as
    /// the user identifies on additional devices or reinstalls.
    #[serde(default)]
    pub install_ids: Vec<String>,
    /// Timestamp of the first identify event that created this row.
    pub identified_at: DateTime,
    /// Most recent activity timestamp (any event from any bound install).
    pub last_seen_at: DateTime,
    // ── Latest known device/app context (populated as events flow). ──
    // Phase 2 will populate these from the SDK's context payload. For now
    // they exist on the schema so adding them later isn't a migration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_app_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_platform: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_os_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_device_model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_device_manufacturer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_locale: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_region: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_timezone: Option<String>,
}

/// Outcome of `AppUsersRepository::upsert_with_install`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppUserUpsert {
    /// First identify for this user_id — created a fresh row with the
    /// install_id in `install_ids`.
    Created,
    /// User existed; added a new install_id to `install_ids`.
    InstallAdded,
    /// User existed and already had this install_id. No change.
    AlreadyPresent,
}
