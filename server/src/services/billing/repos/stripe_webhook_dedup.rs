//! Idempotency for Stripe webhooks.
//!
//! Stripe retries webhooks on 4xx/5xx for up to 72 hours, and also sends
//! occasional duplicates in normal operation. We insert the `event.id` into
//! a dedicated collection with a unique index; a duplicate-key error means
//! "already processed, no-op and return 200."

use async_trait::async_trait;
use mongodb::bson::{self, doc};
use mongodb::error::ErrorKind;
use mongodb::options::IndexOptions;
use mongodb::{Collection, Database};

use crate::ensure_index;
pub use crate::services::billing::models::StripeDedupDoc;

/// Try to record a Stripe event as processed. Returns `Ok(true)` if this is
/// the first time we've seen the event (the caller should then apply it),
/// `Ok(false)` if it's a duplicate.
#[async_trait]
pub trait StripeWebhookDedupRepository: Send + Sync {
    async fn mark_processed(&self, event_id: &str) -> Result<bool, String>;
}

crate::impl_container!(StripeWebhookDedupRepo);
#[derive(Clone)]
pub struct StripeWebhookDedupRepo {
    col: Collection<StripeDedupDoc>,
}

impl StripeWebhookDedupRepo {
    pub async fn new(database: &Database) -> Self {
        let col = database.collection::<StripeDedupDoc>("stripe_webhook_dedup");
        // 30-day TTL matches Stripe's replay window of 72h comfortably and
        // keeps the collection bounded.
        ensure_index!(
            col,
            doc! { "inserted_at": 1 },
            IndexOptions::builder()
                .expire_after(std::time::Duration::from_secs(30 * 24 * 3600))
                .build(),
            "stripe_dedup_ttl"
        );
        StripeWebhookDedupRepo { col }
    }
}

fn is_duplicate_key(e: &mongodb::error::Error) -> bool {
    matches!(e.kind.as_ref(), ErrorKind::Write(_) if e.to_string().contains("E11000"))
}

#[async_trait]
impl StripeWebhookDedupRepository for StripeWebhookDedupRepo {
    async fn mark_processed(&self, event_id: &str) -> Result<bool, String> {
        let doc = StripeDedupDoc {
            event_id: event_id.to_string(),
            inserted_at: bson::DateTime::now(),
        };
        match self.col.insert_one(&doc).await {
            Ok(_) => Ok(true),
            Err(e) if is_duplicate_key(&e) => Ok(false),
            Err(e) => Err(e.to_string()),
        }
    }
}
