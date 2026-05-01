//! Data types for `services/auth/usage/` — request usage logging document.

use mongodb::bson::{self, oid::ObjectId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageDoc {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub api_key_id: Option<ObjectId>,
    pub ip: String,
    pub endpoint: String,
    pub ts: bson::DateTime,
}
