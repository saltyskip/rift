use async_trait::async_trait;
use mongodb::bson::{doc, oid::ObjectId};
use mongodb::options::IndexOptions;
use mongodb::{Collection, Database};

use super::models::UserDoc;
use crate::ensure_index;

// ── Trait ──

#[async_trait]
pub trait UsersRepository: Send + Sync {
    async fn create(&self, doc: &UserDoc) -> Result<(), String>;
    async fn find_by_email(&self, email: &str) -> Result<Option<UserDoc>, String>;
    async fn find_by_tenant_and_email(
        &self,
        tenant_id: &ObjectId,
        email: &str,
    ) -> Result<Option<UserDoc>, String>;
    async fn list_by_tenant(&self, tenant_id: &ObjectId) -> Result<Vec<UserDoc>, String>;
    async fn count_verified_by_tenant(&self, tenant_id: &ObjectId) -> Result<i64, String>;
    async fn delete(&self, tenant_id: &ObjectId, user_id: &ObjectId) -> Result<bool, String>;
    /// Flip `verified: true` on the user with this email. Returns the updated
    /// doc on success, `None` if no such user exists. Idempotent — verifying
    /// an already-verified user is a no-op that returns the doc.
    async fn mark_verified(&self, email: &str) -> Result<Option<UserDoc>, String>;
    async fn upsert_by_email(&self, doc: &UserDoc) -> Result<(), String>;
}

// ── Repository ──

#[derive(Clone)]
pub struct UsersRepo {
    users: Collection<UserDoc>,
}

impl UsersRepo {
    pub async fn new(database: &Database) -> Self {
        let users = database.collection::<UserDoc>("users");

        ensure_index!(
            users,
            doc! { "email": 1 },
            IndexOptions::builder().unique(true).build(),
            "users_email_unique"
        );
        ensure_index!(users, doc! { "tenant_id": 1 }, "users_tenant");

        UsersRepo { users }
    }
}

#[async_trait]
impl UsersRepository for UsersRepo {
    async fn create(&self, doc: &UserDoc) -> Result<(), String> {
        self.users
            .insert_one(doc)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<UserDoc>, String> {
        self.users
            .find_one(doc! { "email": email })
            .await
            .map_err(|e| e.to_string())
    }

    async fn find_by_tenant_and_email(
        &self,
        tenant_id: &ObjectId,
        email: &str,
    ) -> Result<Option<UserDoc>, String> {
        self.users
            .find_one(doc! { "tenant_id": tenant_id, "email": email })
            .await
            .map_err(|e| e.to_string())
    }

    async fn list_by_tenant(&self, tenant_id: &ObjectId) -> Result<Vec<UserDoc>, String> {
        let mut cursor = self
            .users
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

    async fn count_verified_by_tenant(&self, tenant_id: &ObjectId) -> Result<i64, String> {
        self.users
            .count_documents(doc! { "tenant_id": tenant_id, "verified": true })
            .await
            .map(|c| c as i64)
            .map_err(|e| e.to_string())
    }

    async fn delete(&self, tenant_id: &ObjectId, user_id: &ObjectId) -> Result<bool, String> {
        let result = self
            .users
            .delete_one(doc! { "_id": user_id, "tenant_id": tenant_id })
            .await
            .map_err(|e| e.to_string())?;
        Ok(result.deleted_count > 0)
    }

    async fn mark_verified(&self, email: &str) -> Result<Option<UserDoc>, String> {
        self.users
            .find_one_and_update(
                doc! { "email": email },
                doc! { "$set": { "verified": true } },
            )
            .with_options(
                mongodb::options::FindOneAndUpdateOptions::builder()
                    .return_document(mongodb::options::ReturnDocument::After)
                    .build(),
            )
            .await
            .map_err(|e| e.to_string())
    }

    async fn upsert_by_email(&self, doc: &UserDoc) -> Result<(), String> {
        let opts = mongodb::options::UpdateOptions::builder()
            .upsert(true)
            .build();
        self.users
            .update_one(
                doc! { "email": &doc.email },
                doc! {
                    "$set": {
                        "tenant_id": doc.tenant_id,
                        "is_owner": doc.is_owner,
                    },
                    "$setOnInsert": {
                        "email": &doc.email,
                        "verified": doc.verified,
                        "created_at": doc.created_at,
                    },
                },
            )
            .with_options(opts)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}
