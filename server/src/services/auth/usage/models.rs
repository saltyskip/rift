//! Data types for `services/auth/usage/` — request usage logging document.

use mongodb::bson;
use serde::{Deserialize, Serialize};

use crate::core::public_id::SecretKeyId;

// Use a separate marker for usage rows since the `_id` is an internal log row id,
// not a tenant/user/etc. identifier.
crate::impl_container!(UsageRowIdMarker);
pub struct UsageRowIdMarker;
impl crate::core::public_id::IdPrefix for UsageRowIdMarker {
    const PREFIX: &'static str = "usage";
    const SCHEMA_NAME: &'static str = "UsageRowId";
}
pub type UsageRowId = crate::core::public_id::Id<UsageRowIdMarker>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageDoc {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<UsageRowId>,
    pub api_key_id: Option<SecretKeyId>,
    pub ip: String,
    pub endpoint: String,
    pub ts: bson::DateTime,
}
