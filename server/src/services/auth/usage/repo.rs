use async_trait::async_trait;
use chrono::{DateTime, Utc};
use mongodb::bson::{self, doc};
use mongodb::options::IndexOptions;
use mongodb::{Collection, Database};

pub use super::models::UsageDoc;
use crate::ensure_index;

fn to_bson_dt(dt: DateTime<Utc>) -> bson::DateTime {
    bson::DateTime::from_millis(dt.timestamp_millis())
}

pub fn now_bson() -> bson::DateTime {
    to_bson_dt(Utc::now())
}

// ── Trait ──

#[async_trait]
pub trait UsageRepository: Send + Sync {
    async fn record_usage(&self, usage_doc: UsageDoc);
    async fn count_ip_usage_since(&self, ip: &str, since: DateTime<Utc>) -> i64;
}

// ── Repository ──

crate::impl_container!(UsageRepo);
#[derive(Clone)]
pub struct UsageRepo {
    usage: Collection<UsageDoc>,
}

impl UsageRepo {
    pub async fn new(database: &Database) -> Self {
        let usage = database.collection::<UsageDoc>("usage");

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

        UsageRepo { usage }
    }
}

#[async_trait]
impl UsageRepository for UsageRepo {
    async fn record_usage(&self, usage_doc: UsageDoc) {
        let _ = self.usage.insert_one(usage_doc).await;
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
