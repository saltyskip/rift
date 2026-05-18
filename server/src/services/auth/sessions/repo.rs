//! Sessions collection — storage + lookup primitives.
//!
//! Two indexes:
//! - `{ token_hash: 1 }` unique — every session lookup hits this index
//! - `{ user_id: 1, expires_at: 1 }` — used for listing / revoking a user's sessions
//! - TTL on `expires_at` so MongoDB sweeps expired sessions automatically

use async_trait::async_trait;
use mongodb::bson::{self, doc, oid::ObjectId};
use mongodb::options::IndexOptions;
use mongodb::{Collection, Database};

use super::models::SessionDoc;
use crate::ensure_index;

#[async_trait]
pub trait SessionsRepository: Send + Sync {
    async fn insert(&self, doc: &SessionDoc) -> Result<(), String>;

    /// Find an active session by hashed token. Returns the doc if the session
    /// exists, has not been revoked, and has not expired. Returns `Ok(None)` in
    /// any of those cases.
    async fn find_active_by_hash(&self, token_hash: &str) -> Result<Option<SessionDoc>, String>;

    /// Mark `revoked_at = now` for a single session. Idempotent: revoking an
    /// already-revoked session is a no-op.
    async fn revoke(&self, session_id: &ObjectId) -> Result<bool, String>;

    /// Bump `last_seen_at = now`. Caller is expected to debounce (the middleware
    /// only calls this when `now - last_seen > 60s`).
    async fn touch_last_seen(&self, session_id: &ObjectId) -> Result<(), String>;
}

// ── Mongo impl ──

crate::impl_container!(SessionsRepoMongo);
#[derive(Clone)]
pub struct SessionsRepoMongo {
    col: Collection<SessionDoc>,
}

impl SessionsRepoMongo {
    pub async fn new(database: &Database) -> Self {
        let col = database.collection::<SessionDoc>("sessions");

        ensure_index!(
            col,
            doc! { "token_hash": 1 },
            IndexOptions::builder().unique(true).build(),
            "sessions_token_hash_unique"
        );
        ensure_index!(
            col,
            doc! { "user_id": 1, "expires_at": 1 },
            "sessions_user_expires"
        );
        ensure_index!(
            col,
            doc! { "expires_at": 1 },
            IndexOptions::builder()
                .expire_after(std::time::Duration::from_secs(0))
                .build(),
            "sessions_ttl"
        );

        SessionsRepoMongo { col }
    }
}

#[async_trait]
impl SessionsRepository for SessionsRepoMongo {
    async fn insert(&self, doc: &SessionDoc) -> Result<(), String> {
        self.col.insert_one(doc).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    async fn find_active_by_hash(&self, token_hash: &str) -> Result<Option<SessionDoc>, String> {
        let now = bson::DateTime::now();
        self.col
            .find_one(doc! {
                "token_hash": token_hash,
                "revoked_at": null,
                "expires_at": { "$gt": now },
            })
            .await
            .map_err(|e| e.to_string())
    }

    async fn revoke(&self, session_id: &ObjectId) -> Result<bool, String> {
        let now = bson::DateTime::now();
        let result = self
            .col
            .update_one(
                doc! { "_id": session_id, "revoked_at": null },
                doc! { "$set": { "revoked_at": now } },
            )
            .await
            .map_err(|e| e.to_string())?;
        Ok(result.modified_count > 0)
    }

    async fn touch_last_seen(&self, session_id: &ObjectId) -> Result<(), String> {
        let now = bson::DateTime::now();
        self.col
            .update_one(
                doc! { "_id": session_id },
                doc! { "$set": { "last_seen_at": now } },
            )
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}
