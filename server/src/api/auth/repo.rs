use async_trait::async_trait;
use chrono::{DateTime, Utc};
use mongodb::bson::{self, doc, oid::ObjectId};
use mongodb::options::{IndexOptions, UpdateOptions};
use mongodb::{Collection, Database};
use serde::{Deserialize, Serialize};

use crate::ensure_index;

// ── Documents ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyDoc {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub email: String,
    pub key_hash: String,
    pub key_prefix: String,
    pub verified: bool,
    pub verify_token: Option<String>,
    pub monthly_quota: i64,
    pub created_at: bson::DateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageDoc {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub api_key_id: Option<ObjectId>,
    pub ip: String,
    pub endpoint: String,
    pub ts: bson::DateTime,
}

// ── Conversions ──

fn to_bson_dt(dt: DateTime<Utc>) -> bson::DateTime {
    bson::DateTime::from_millis(dt.timestamp_millis())
}

pub fn now_bson() -> bson::DateTime {
    to_bson_dt(Utc::now())
}

// ── Trait ──

#[async_trait]
pub trait AuthRepository: Send + Sync {
    async fn find_key_by_hash(&self, hash: &str) -> Option<ApiKeyDoc>;
    async fn find_key_by_email(&self, email: &str) -> Option<ApiKeyDoc>;
    async fn upsert_key(&self, key_doc: &ApiKeyDoc) -> Result<(), String>;
    async fn verify_key(&self, token: &str) -> bool;
    async fn record_usage(&self, usage_doc: UsageDoc);
    async fn count_key_usage_since(&self, key_id: &ObjectId, since: DateTime<Utc>) -> i64;
    async fn count_ip_usage_since(&self, ip: &str, since: DateTime<Utc>) -> i64;
}

// ── Repository ──

#[derive(Clone)]
pub struct AuthRepo {
    pub api_keys: Collection<ApiKeyDoc>,
    pub usage: Collection<UsageDoc>,
}

impl AuthRepo {
    pub async fn new(database: &Database) -> Self {
        let api_keys = database.collection::<ApiKeyDoc>("api_keys");
        let usage = database.collection::<UsageDoc>("usage");

        ensure_index!(
            api_keys,
            doc! { "email": 1 },
            IndexOptions::builder().unique(true).build(),
            "email_unique"
        );
        ensure_index!(
            api_keys,
            doc! { "key_hash": 1 },
            IndexOptions::builder().unique(true).build(),
            "key_hash_unique"
        );
        ensure_index!(
            usage,
            doc! { "ts": 1 },
            IndexOptions::builder()
                .expire_after(std::time::Duration::from_secs(60 * 24 * 3600))
                .build(),
            "usage_ttl"
        );
        ensure_index!(usage, doc! { "api_key_id": 1, "ts": 1 }, "usage_key_ts");
        ensure_index!(usage, doc! { "ip": 1, "ts": 1 }, "usage_ip_ts");

        AuthRepo { api_keys, usage }
    }
}

#[async_trait]
impl AuthRepository for AuthRepo {
    async fn find_key_by_hash(&self, hash: &str) -> Option<ApiKeyDoc> {
        self.api_keys
            .find_one(doc! { "key_hash": hash, "verified": true })
            .await
            .ok()
            .flatten()
    }

    async fn find_key_by_email(&self, email: &str) -> Option<ApiKeyDoc> {
        self.api_keys
            .find_one(doc! { "email": email })
            .await
            .ok()
            .flatten()
    }

    async fn upsert_key(&self, key_doc: &ApiKeyDoc) -> Result<(), String> {
        let opts = UpdateOptions::builder().upsert(true).build();
        self.api_keys
            .update_one(
                doc! { "email": &key_doc.email },
                doc! {
                    "$set": {
                        "key_hash": &key_doc.key_hash,
                        "key_prefix": &key_doc.key_prefix,
                        "verified": key_doc.verified,
                        "verify_token": &key_doc.verify_token,
                        "monthly_quota": key_doc.monthly_quota,
                    },
                    "$setOnInsert": {
                        "email": &key_doc.email,
                        "created_at": key_doc.created_at,
                    },
                },
            )
            .with_options(opts)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    async fn verify_key(&self, token: &str) -> bool {
        let result = self
            .api_keys
            .update_one(
                doc! { "verify_token": token, "verified": false },
                doc! {
                    "$set": { "verified": true },
                    "$unset": { "verify_token": "" },
                },
            )
            .await;
        matches!(result, Ok(r) if r.modified_count > 0)
    }

    async fn record_usage(&self, usage_doc: UsageDoc) {
        let _ = self.usage.insert_one(usage_doc).await;
    }

    async fn count_key_usage_since(&self, key_id: &ObjectId, since: DateTime<Utc>) -> i64 {
        self.usage
            .count_documents(doc! {
                "api_key_id": key_id,
                "ts": { "$gte": to_bson_dt(since) },
            })
            .await
            .unwrap_or(0) as i64
    }

    async fn count_ip_usage_since(&self, ip: &str, since: DateTime<Utc>) -> i64 {
        self.usage
            .count_documents(doc! {
                "api_key_id": null,
                "ip": ip,
                "ts": { "$gte": to_bson_dt(since) },
            })
            .await
            .unwrap_or(0) as i64
    }
}
