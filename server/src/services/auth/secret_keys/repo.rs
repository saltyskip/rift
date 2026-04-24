use async_trait::async_trait;
use mongodb::bson::{self, doc, oid::ObjectId};
use mongodb::options::IndexOptions;
use mongodb::{Collection, Database};
use serde::{Deserialize, Serialize};

use crate::ensure_index;

// ── Document ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretKeyDoc {
    #[serde(rename = "_id")]
    pub id: ObjectId,
    pub tenant_id: ObjectId,
    pub created_by: ObjectId,
    pub key_hash: String,
    pub key_prefix: String,
    pub created_at: bson::DateTime,
}

// ── Trait ──
//
// Rotation-request plumbing (attempts counter, TTL'd requests collection)
// used to live here. It moved to `services/tokens` — `SecretKeysService`
// now talks to `TokenService` directly for the request/confirm dance.

#[async_trait]
pub trait SecretKeysRepository: Send + Sync {
    async fn create_key(&self, doc: &SecretKeyDoc) -> Result<(), String>;
    async fn find_by_hash(&self, key_hash: &str) -> Result<Option<SecretKeyDoc>, String>;
    async fn list_by_tenant(&self, tenant_id: &ObjectId) -> Result<Vec<SecretKeyDoc>, String>;
    async fn count_by_tenant(&self, tenant_id: &ObjectId) -> Result<i64, String>;
    async fn delete_key(&self, tenant_id: &ObjectId, key_id: &ObjectId) -> Result<bool, String>;
}

// ── Repository ──

#[derive(Clone)]
pub struct SecretKeysRepo {
    keys: Collection<SecretKeyDoc>,
}

impl SecretKeysRepo {
    pub async fn new(database: &Database) -> Self {
        let keys = database.collection::<SecretKeyDoc>("secret_keys");

        ensure_index!(
            keys,
            doc! { "key_hash": 1 },
            IndexOptions::builder().unique(true).build(),
            "secret_key_hash_unique"
        );
        ensure_index!(keys, doc! { "tenant_id": 1 }, "secret_keys_tenant");

        SecretKeysRepo { keys }
    }
}

#[async_trait]
impl SecretKeysRepository for SecretKeysRepo {
    async fn create_key(&self, doc: &SecretKeyDoc) -> Result<(), String> {
        self.keys.insert_one(doc).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    async fn find_by_hash(&self, key_hash: &str) -> Result<Option<SecretKeyDoc>, String> {
        self.keys
            .find_one(doc! { "key_hash": key_hash })
            .await
            .map_err(|e| e.to_string())
    }

    async fn list_by_tenant(&self, tenant_id: &ObjectId) -> Result<Vec<SecretKeyDoc>, String> {
        let mut cursor = self
            .keys
            .find(doc! { "tenant_id": tenant_id })
            .sort(doc! { "created_at": -1 })
            .await
            .map_err(|e| e.to_string())?;

        let mut docs = Vec::new();
        while cursor.advance().await.map_err(|e| e.to_string())? {
            docs.push(cursor.deserialize_current().map_err(|e| e.to_string())?);
        }
        Ok(docs)
    }

    async fn count_by_tenant(&self, tenant_id: &ObjectId) -> Result<i64, String> {
        self.keys
            .count_documents(doc! { "tenant_id": tenant_id })
            .await
            .map(|c| c as i64)
            .map_err(|e| e.to_string())
    }

    async fn delete_key(&self, tenant_id: &ObjectId, key_id: &ObjectId) -> Result<bool, String> {
        let result = self
            .keys
            .delete_one(doc! { "_id": key_id, "tenant_id": tenant_id })
            .await
            .map_err(|e| e.to_string())?;
        Ok(result.deleted_count > 0)
    }
}
