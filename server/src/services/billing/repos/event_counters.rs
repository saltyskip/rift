use async_trait::async_trait;
use mongodb::bson::{self, doc, oid::ObjectId};
use mongodb::error::ErrorKind;
use mongodb::options::{FindOneAndUpdateOptions, IndexOptions, ReturnDocument};
use mongodb::{Collection, Database};
use serde::{Deserialize, Serialize};

use crate::ensure_index;

// ── Document ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventCounterDoc {
    #[serde(rename = "_id")]
    pub id: String,
    pub tenant_id: ObjectId,
    pub period: String, // e.g. "2026-04"
    pub count: i64,
    pub created_at: bson::DateTime,
}

// ── Trait ──

/// Atomic per-tenant-per-month event counter. Powers quota enforcement on the
/// click + conversion hot paths without a `count_documents` query per event.
///
/// `increment_if_below` is race-free via `findOneAndUpdate` with a conditional
/// filter + upsert (see Gate C in the billing plan). The two rejection paths
/// are: filter didn't match existing doc (at/over limit) or upsert collided
/// with existing doc at limit (duplicate key).
#[async_trait]
pub trait EventCountersRepository: Send + Sync {
    async fn increment_if_below(
        &self,
        tenant_id: &ObjectId,
        period: &str,
        max: Option<u64>,
    ) -> Result<bool, String>;
}

// ── MongoDB impl ──

#[derive(Clone)]
pub struct EventCountersRepo {
    counters: Collection<EventCounterDoc>,
}

impl EventCountersRepo {
    pub async fn new(database: &Database) -> Self {
        let counters = database.collection::<EventCounterDoc>("tenant_event_counters");
        // 90-day TTL on `created_at` keeps the collection bounded. Two full
        // months of active counters + a buffer.
        ensure_index!(
            counters,
            doc! { "created_at": 1 },
            IndexOptions::builder()
                .expire_after(std::time::Duration::from_secs(90 * 24 * 3600))
                .build(),
            "counters_ttl"
        );
        EventCountersRepo { counters }
    }
}

fn counter_id(tenant_id: &ObjectId, period: &str) -> String {
    format!("{}:{}:events", tenant_id.to_hex(), period)
}

fn is_duplicate_key(e: &mongodb::error::Error) -> bool {
    matches!(e.kind.as_ref(), ErrorKind::Write(_) if e.to_string().contains("E11000"))
}

#[async_trait]
impl EventCountersRepository for EventCountersRepo {
    async fn increment_if_below(
        &self,
        tenant_id: &ObjectId,
        period: &str,
        max: Option<u64>,
    ) -> Result<bool, String> {
        let Some(max) = max else { return Ok(true) };
        let id = counter_id(tenant_id, period);
        let filter = doc! {
            "_id": &id,
            "count": { "$lt": max as i64 },
        };
        let update = doc! {
            "$inc": { "count": 1i64 },
            "$setOnInsert": {
                "tenant_id": tenant_id,
                "period": period,
                "created_at": bson::DateTime::now(),
            },
        };
        let opts = FindOneAndUpdateOptions::builder()
            .upsert(true)
            .return_document(ReturnDocument::After)
            .build();

        match self
            .counters
            .find_one_and_update(filter, update)
            .with_options(opts)
            .await
        {
            Ok(Some(_)) => Ok(true),
            // No match + upsert couldn't create one (would only happen if
            // the driver returns None on upsert-race; defensively treat as
            // over-limit).
            Ok(None) => Ok(false),
            // Upsert tried to insert a doc with existing _id — caller is at
            // or above the limit.
            Err(e) if is_duplicate_key(&e) => Ok(false),
            Err(e) => Err(e.to_string()),
        }
    }
}
