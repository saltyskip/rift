//! Tokens collection — storage + atomic consume primitives.

use async_trait::async_trait;
use mongodb::bson::{self, doc, oid::ObjectId};
use mongodb::options::IndexOptions;
use mongodb::{Collection, Database};

use super::models::{TokenDoc, TokenPurpose};
use crate::ensure_index;

#[async_trait]
pub trait TokensRepository: Send + Sync {
    /// Insert a fresh token doc.
    async fn insert(&self, doc: &TokenDoc) -> Result<(), String>;

    /// Remove any pending (purpose, email) tuple-keyed docs. Called by
    /// `TokenService::issue` for TupleKeyed to guarantee at most one active
    /// code per (purpose, email) — matches the old `secret_key_create_requests`
    /// semantics where a fresh request supersedes the previous one.
    async fn delete_pending_tuple(&self, purpose: TokenPurpose, email: &str)
        -> Result<u64, String>;

    /// Atomic single-use consume by token hash. Marks `consumed_at` on
    /// success and returns the doc (with purpose + metadata). Returns
    /// `Ok(None)` if not found / expired / already consumed.
    async fn consume_by_hash(&self, token_hash: &str) -> Result<Option<TokenDoc>, String>;

    /// Find the pending tuple-keyed doc for (purpose, email), increment
    /// `attempts` by 1, return the updated doc. Returns `Ok(None)` if no
    /// such doc exists.
    async fn increment_tuple_attempts(
        &self,
        purpose: TokenPurpose,
        email: &str,
    ) -> Result<Option<TokenDoc>, String>;

    /// Delete the pending tuple-keyed doc and return it (used on successful
    /// consume OR on attempts-exhausted cleanup).
    async fn delete_tuple(
        &self,
        purpose: TokenPurpose,
        email: &str,
    ) -> Result<Option<TokenDoc>, String>;

    /// Count how many (purpose, email) tokens were created in the last
    /// `window_secs`. Used for per-email rate limiting.
    async fn count_recent_for_email(
        &self,
        purpose: TokenPurpose,
        email: &str,
        window_secs: i64,
    ) -> Result<u64, String>;

    /// True if there's an unexpired, unconsumed (purpose, email) doc —
    /// used for cooldown checks (e.g. "a key rotation request is already
    /// pending").
    async fn pending_exists(&self, purpose: TokenPurpose, email: &str) -> Result<bool, String>;
}

// ── Mongo impl ──

#[derive(Clone)]
pub struct TokensRepoMongo {
    col: Collection<TokenDoc>,
}

impl TokensRepoMongo {
    pub async fn new(database: &Database) -> Self {
        let col = database.collection::<TokenDoc>("tokens");

        ensure_index!(
            col,
            doc! { "expires_at": 1 },
            IndexOptions::builder()
                .expire_after(std::time::Duration::from_secs(0))
                .build(),
            "tokens_ttl"
        );
        ensure_index!(
            col,
            doc! { "purpose": 1, "email": 1, "created_at": -1 },
            "tokens_purpose_email"
        );

        TokensRepoMongo { col }
    }
}

fn bson_now() -> bson::DateTime {
    bson::DateTime::now()
}

#[async_trait]
impl TokensRepository for TokensRepoMongo {
    async fn insert(&self, doc: &TokenDoc) -> Result<(), String> {
        self.col.insert_one(doc).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    async fn delete_pending_tuple(
        &self,
        purpose: TokenPurpose,
        email: &str,
    ) -> Result<u64, String> {
        let result = self
            .col
            .delete_many(doc! {
                "purpose": bson::to_bson(&purpose).map_err(|e| e.to_string())?,
                "email": email,
                "consumed_at": null,
            })
            .await
            .map_err(|e| e.to_string())?;
        Ok(result.deleted_count)
    }

    async fn consume_by_hash(&self, token_hash: &str) -> Result<Option<TokenDoc>, String> {
        let now = bson_now();
        self.col
            .find_one_and_update(
                doc! {
                    "_id": token_hash,
                    "consumed_at": null,
                    "expires_at": { "$gt": now },
                },
                doc! { "$set": { "consumed_at": now } },
            )
            .await
            .map_err(|e| e.to_string())
    }

    async fn increment_tuple_attempts(
        &self,
        purpose: TokenPurpose,
        email: &str,
    ) -> Result<Option<TokenDoc>, String> {
        let now = bson_now();
        self.col
            .find_one_and_update(
                doc! {
                    "purpose": bson::to_bson(&purpose).map_err(|e| e.to_string())?,
                    "email": email,
                    "consumed_at": null,
                    "expires_at": { "$gt": now },
                },
                doc! { "$inc": { "attempts": 1 } },
            )
            .with_options(
                mongodb::options::FindOneAndUpdateOptions::builder()
                    .return_document(mongodb::options::ReturnDocument::After)
                    .build(),
            )
            .await
            .map_err(|e| e.to_string())
    }

    async fn delete_tuple(
        &self,
        purpose: TokenPurpose,
        email: &str,
    ) -> Result<Option<TokenDoc>, String> {
        self.col
            .find_one_and_delete(doc! {
                "purpose": bson::to_bson(&purpose).map_err(|e| e.to_string())?,
                "email": email,
            })
            .await
            .map_err(|e| e.to_string())
    }

    async fn count_recent_for_email(
        &self,
        purpose: TokenPurpose,
        email: &str,
        window_secs: i64,
    ) -> Result<u64, String> {
        let since =
            bson::DateTime::from_millis(chrono::Utc::now().timestamp_millis() - window_secs * 1000);
        self.col
            .count_documents(doc! {
                "purpose": bson::to_bson(&purpose).map_err(|e| e.to_string())?,
                "email": email,
                "created_at": { "$gte": since },
            })
            .await
            .map_err(|e| e.to_string())
    }

    async fn pending_exists(&self, purpose: TokenPurpose, email: &str) -> Result<bool, String> {
        let now = bson_now();
        let count = self
            .col
            .count_documents(doc! {
                "purpose": bson::to_bson(&purpose).map_err(|e| e.to_string())?,
                "email": email,
                "consumed_at": null,
                "expires_at": { "$gt": now },
            })
            .await
            .map_err(|e| e.to_string())?;
        Ok(count > 0)
    }
}

// ── Helpers ──

/// Build a fresh doc. Callers go through `TokenService::issue` which wraps
/// this; exposed here for tests that want to construct docs directly.
pub fn new_token_doc(
    id: String,
    purpose: TokenPurpose,
    email: String,
    token_hash: String,
    metadata: bson::Document,
    max_attempts: i32,
    ttl_secs: i64,
) -> TokenDoc {
    TokenDoc {
        id,
        purpose,
        email,
        token_hash,
        metadata,
        attempts: 0,
        max_attempts,
        expires_at: bson::DateTime::from_millis(
            chrono::Utc::now().timestamp_millis() + ttl_secs * 1000,
        ),
        consumed_at: None,
        created_at: bson::DateTime::now(),
    }
}

/// Convenience: random ObjectId hex, used as the `_id` of tuple-keyed docs
/// where the raw code is too short to use as a primary key.
pub fn random_doc_id() -> String {
    ObjectId::new().to_hex()
}
