use async_trait::async_trait;
use mongodb::bson::{self, doc, oid::ObjectId};
use mongodb::options::IndexOptions;
use mongodb::{Collection, Database};
use serde::{Deserialize, Serialize};

use crate::ensure_index;

// ── Documents ──

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretKeyCreateRequestDoc {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub tenant_id: ObjectId,
    pub user_id: ObjectId,
    pub token_hash: String,
    pub attempts: i32,
    pub expires_at: bson::DateTime,
    pub created_at: bson::DateTime,
}

// ── Trait ──

#[async_trait]
pub trait SecretKeysRepository: Send + Sync {
    async fn create_key(&self, doc: &SecretKeyDoc) -> Result<(), String>;
    async fn find_by_hash(&self, key_hash: &str) -> Result<Option<SecretKeyDoc>, String>;
    async fn list_by_tenant(&self, tenant_id: &ObjectId) -> Result<Vec<SecretKeyDoc>, String>;
    async fn count_by_tenant(&self, tenant_id: &ObjectId) -> Result<i64, String>;
    async fn delete_key(&self, tenant_id: &ObjectId, key_id: &ObjectId) -> Result<bool, String>;

    // Key creation request methods
    async fn find_pending_request(
        &self,
        tenant_id: &ObjectId,
        user_id: &ObjectId,
    ) -> Result<Option<SecretKeyCreateRequestDoc>, String>;
    async fn create_request(&self, doc: &SecretKeyCreateRequestDoc) -> Result<(), String>;
    async fn validate_and_consume_request(
        &self,
        tenant_id: &ObjectId,
        user_id: &ObjectId,
        token_hash: &str,
    ) -> Result<bool, String>;
    async fn increment_request_attempts(
        &self,
        tenant_id: &ObjectId,
        user_id: &ObjectId,
    ) -> Result<i32, String>;
}

// ── Repository ──

#[derive(Clone)]
pub struct SecretKeysRepo {
    keys: Collection<SecretKeyDoc>,
    requests: Collection<SecretKeyCreateRequestDoc>,
}

impl SecretKeysRepo {
    pub async fn new(database: &Database) -> Self {
        let keys = database.collection::<SecretKeyDoc>("secret_keys");
        let requests =
            database.collection::<SecretKeyCreateRequestDoc>("secret_key_create_requests");

        ensure_index!(
            keys,
            doc! { "key_hash": 1 },
            IndexOptions::builder().unique(true).build(),
            "secret_key_hash_unique"
        );
        ensure_index!(keys, doc! { "tenant_id": 1 }, "secret_keys_tenant");

        ensure_index!(
            requests,
            doc! { "tenant_id": 1, "user_id": 1 },
            "sk_requests_tenant_user"
        );
        ensure_index!(
            requests,
            doc! { "expires_at": 1 },
            IndexOptions::builder()
                .expire_after(std::time::Duration::from_secs(0))
                .build(),
            "sk_requests_ttl"
        );

        SecretKeysRepo { keys, requests }
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

    async fn find_pending_request(
        &self,
        tenant_id: &ObjectId,
        user_id: &ObjectId,
    ) -> Result<Option<SecretKeyCreateRequestDoc>, String> {
        let now = bson::DateTime::now();
        self.requests
            .find_one(doc! {
                "tenant_id": tenant_id,
                "user_id": user_id,
                "expires_at": { "$gt": now },
            })
            .await
            .map_err(|e| e.to_string())
    }

    async fn create_request(&self, doc: &SecretKeyCreateRequestDoc) -> Result<(), String> {
        // Delete any existing request for this tenant+user before inserting
        let _ = self
            .requests
            .delete_many(doc! { "tenant_id": doc.tenant_id, "user_id": doc.user_id })
            .await;
        self.requests
            .insert_one(doc)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    async fn validate_and_consume_request(
        &self,
        tenant_id: &ObjectId,
        user_id: &ObjectId,
        token_hash: &str,
    ) -> Result<bool, String> {
        let now = bson::DateTime::now();
        let result = self
            .requests
            .delete_one(doc! {
                "tenant_id": tenant_id,
                "user_id": user_id,
                "token_hash": token_hash,
                "expires_at": { "$gt": now },
                "attempts": { "$lt": 5 },
            })
            .await
            .map_err(|e| e.to_string())?;
        Ok(result.deleted_count > 0)
    }

    async fn increment_request_attempts(
        &self,
        tenant_id: &ObjectId,
        user_id: &ObjectId,
    ) -> Result<i32, String> {
        let now = bson::DateTime::now();
        let result = self
            .requests
            .find_one_and_update(
                doc! {
                    "tenant_id": tenant_id,
                    "user_id": user_id,
                    "expires_at": { "$gt": now },
                },
                doc! { "$inc": { "attempts": 1 } },
            )
            .await
            .map_err(|e| e.to_string())?;
        Ok(result.map(|d| d.attempts + 1).unwrap_or(0))
    }
}
