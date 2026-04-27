use mongodb::bson::{self, oid::ObjectId};
use serde::{Deserialize, Serialize};

/// Stored secret key (`rl_live_…`).
///
/// `scope` is optional only as a migration-window concession — pre-existing
/// rows in production deserialize as `None`. The auth middleware grandfathers
/// `None` to `KeyScope::Full` for one release cycle. Migration `m004` backfills
/// `Some(KeyScope::Full)` on all such rows; a follow-up PR will then make this
/// field required (non-Option) and flip middleware to reject `None`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretKeyDoc {
    #[serde(rename = "_id")]
    pub id: ObjectId,
    pub tenant_id: ObjectId,
    pub created_by: ObjectId,
    pub key_hash: String,
    pub key_prefix: String,
    pub created_at: bson::DateTime,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<KeyScope>,
}

/// Access level a secret key carries.
///
/// Tagged-enum serialization (`{ "type": "full" }` / `{ "type": "affiliate",
/// "affiliate_id": "…" }`) keeps the schema additive — future variants
/// (e.g. `ReadOnly`, `Webhook`) can be added without migrating old rows.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum KeyScope {
    /// Full tenant access. The default for advertiser-issued keys —
    /// can mint links, manage affiliates, domains, webhooks, team, etc.
    Full,
    /// Partner-scoped access. Key can only operate on the named affiliate's
    /// links (mint pinned to this id, read its own links). Cannot manage
    /// tenant resources.
    Affiliate { affiliate_id: ObjectId },
}
