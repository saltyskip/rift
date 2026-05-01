use async_trait::async_trait;
use mongodb::bson::{doc, oid::ObjectId};
use mongodb::options::IndexOptions;
use mongodb::{Collection, Database};

use crate::ensure_index;

use super::models::SdkKeyDoc;

// ── Trait ──

#[async_trait]
pub trait SdkKeysRepository: Send + Sync {
    async fn create_key(&self, doc: &SdkKeyDoc) -> Result<(), String>;

    async fn find_by_hash(&self, key_hash: &str) -> Result<Option<SdkKeyDoc>, String>;

    async fn list_by_tenant(&self, tenant_id: &ObjectId) -> Result<Vec<SdkKeyDoc>, String>;

    async fn revoke(&self, tenant_id: &ObjectId, key_id: &ObjectId) -> Result<bool, String>;
}

// ── Repository ──

crate::impl_container!(SdkKeysRepo);
#[derive(Clone)]
pub struct SdkKeysRepo {
    keys: Collection<SdkKeyDoc>,
}

impl SdkKeysRepo {
    pub async fn new(database: &Database) -> Self {
        let keys = database.collection::<SdkKeyDoc>("sdk_keys");

        ensure_index!(
            keys,
            doc! { "key_hash": 1 },
            IndexOptions::builder().unique(true).build(),
            "sdk_key_hash_unique"
        );
        ensure_index!(keys, doc! { "tenant_id": 1 }, "sdk_keys_tenant");

        SdkKeysRepo { keys }
    }
}

#[async_trait]
impl SdkKeysRepository for SdkKeysRepo {
    async fn create_key(&self, doc: &SdkKeyDoc) -> Result<(), String> {
        self.keys.insert_one(doc).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    async fn find_by_hash(&self, key_hash: &str) -> Result<Option<SdkKeyDoc>, String> {
        self.keys
            .find_one(doc! { "key_hash": key_hash, "revoked": false })
            .await
            .map_err(|e| e.to_string())
    }

    async fn list_by_tenant(&self, tenant_id: &ObjectId) -> Result<Vec<SdkKeyDoc>, String> {
        let mut cursor = self
            .keys
            .find(doc! { "tenant_id": tenant_id, "revoked": false })
            .sort(doc! { "created_at": -1 })
            .await
            .map_err(|e| e.to_string())?;

        let mut docs = Vec::new();
        while cursor.advance().await.map_err(|e| e.to_string())? {
            docs.push(cursor.deserialize_current().map_err(|e| e.to_string())?);
        }
        Ok(docs)
    }

    async fn revoke(&self, tenant_id: &ObjectId, key_id: &ObjectId) -> Result<bool, String> {
        let result = self
            .keys
            .update_one(
                doc! { "_id": key_id, "tenant_id": tenant_id },
                doc! { "$set": { "revoked": true } },
            )
            .await
            .map_err(|e| e.to_string())?;
        Ok(result.modified_count > 0)
    }
}
