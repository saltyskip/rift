//! Billing magic-link tokens.
//!
//! A magic link is an ephemeral credential that proves control of an email
//! address for exactly one billing action (start/upgrade a subscription, or
//! open the Stripe Billing Portal). Tokens are single-use, TTL-expired, and
//! stored hashed — the raw token only exists in the email we send.
//!
//! Design:
//! - `_id` = sha256(raw_token). Hash collisions are infeasible in 256 bits,
//!   and using the hash as the primary key means the only index lookup is
//!   already O(1).
//! - `consumed_at` is set via `find_one_and_update` on redemption. Mongo's
//!   atomic update ensures a token can only be redeemed once even if a user
//!   clicks the link twice.
//! - TTL index on `expires_at` purges stale docs ~15 min after creation.

use async_trait::async_trait;
use mongodb::bson::{self, doc};
use mongodb::options::IndexOptions;
use mongodb::{Collection, Database};
use serde::{Deserialize, Serialize};

use crate::ensure_index;
use crate::services::auth::keys;

/// Which flow a magic link gates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MagicLinkIntent {
    /// Start a new subscription or upgrade an existing tenant. Requires `tier`.
    Subscribe,
    /// Open a Stripe Billing Portal session for an existing customer.
    Portal,
}

/// Tier selection for `Subscribe` intent. Mirrors the paid tiers from
/// `PlanTier` without the Free variant — Free doesn't need a magic link.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MagicLinkTier {
    Pro,
    Business,
    Scale,
}

impl MagicLinkTier {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "pro" => Some(Self::Pro),
            "business" => Some(Self::Business),
            "scale" => Some(Self::Scale),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MagicLinkDoc {
    /// sha256(raw_token), hex-encoded.
    #[serde(rename = "_id")]
    pub token_hash: String,
    pub email: String,
    pub intent: MagicLinkIntent,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tier: Option<MagicLinkTier>,
    pub expires_at: bson::DateTime,
    pub created_at: bson::DateTime,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub consumed_at: Option<bson::DateTime>,
}

#[async_trait]
pub trait MagicLinksRepository: Send + Sync {
    /// Create a new magic link. Returns `(raw_token, stored_doc)`. The raw
    /// token is only ever visible here and in the email we send.
    async fn create(
        &self,
        email: &str,
        intent: MagicLinkIntent,
        tier: Option<MagicLinkTier>,
        ttl_secs: i64,
    ) -> Result<(String, MagicLinkDoc), String>;

    /// Atomically consume a token. Returns the stored doc on first-time use,
    /// None if the token is unknown, already consumed, or expired.
    async fn consume(&self, raw_token: &str) -> Result<Option<MagicLinkDoc>, String>;

    /// Count recently-created links for a given email (for per-email rate
    /// limiting). Counts both consumed and unconsumed docs within the TTL
    /// window.
    async fn count_recent_for_email(&self, email: &str, window_secs: i64) -> Result<u64, String>;
}

#[derive(Clone)]
pub struct MagicLinksRepo {
    col: Collection<MagicLinkDoc>,
}

impl MagicLinksRepo {
    pub async fn new(database: &Database) -> Self {
        let col = database.collection::<MagicLinkDoc>("billing_magic_links");

        ensure_index!(
            col,
            doc! { "expires_at": 1 },
            IndexOptions::builder()
                .expire_after(std::time::Duration::from_secs(0))
                .build(),
            "billing_magic_links_ttl"
        );
        ensure_index!(
            col,
            doc! { "email": 1, "created_at": -1 },
            "billing_magic_links_email"
        );

        MagicLinksRepo { col }
    }
}

#[async_trait]
impl MagicLinksRepository for MagicLinksRepo {
    async fn create(
        &self,
        email: &str,
        intent: MagicLinkIntent,
        tier: Option<MagicLinkTier>,
        ttl_secs: i64,
    ) -> Result<(String, MagicLinkDoc), String> {
        let raw_token = keys::generate_verify_token();
        let token_hash = keys::hash_key(&raw_token);
        let now = bson::DateTime::now();
        let expires_at =
            bson::DateTime::from_millis(chrono::Utc::now().timestamp_millis() + ttl_secs * 1000);

        let doc = MagicLinkDoc {
            token_hash,
            email: email.to_string(),
            intent,
            tier,
            expires_at,
            created_at: now,
            consumed_at: None,
        };
        self.col.insert_one(&doc).await.map_err(|e| e.to_string())?;
        Ok((raw_token, doc))
    }

    async fn consume(&self, raw_token: &str) -> Result<Option<MagicLinkDoc>, String> {
        let token_hash = keys::hash_key(raw_token);
        let now = bson::DateTime::now();
        self.col
            .find_one_and_update(
                doc! {
                    "_id": &token_hash,
                    "consumed_at": null,
                    "expires_at": { "$gt": now },
                },
                doc! { "$set": { "consumed_at": now } },
            )
            .await
            .map_err(|e| e.to_string())
    }

    async fn count_recent_for_email(&self, email: &str, window_secs: i64) -> Result<u64, String> {
        let since =
            bson::DateTime::from_millis(chrono::Utc::now().timestamp_millis() - window_secs * 1000);
        self.col
            .count_documents(doc! {
                "email": email,
                "created_at": { "$gte": since },
            })
            .await
            .map_err(|e| e.to_string())
    }
}
