//! Per-tier retention TTL indexes on time-series event collections.
//!
//! MongoDB time-series collections in 7.0+ support partial TTL indexes that
//! are keyed on the timeField but filter on the metaField. We embed a
//! `retention_bucket` string in each event's meta ("30d" | "1y" | "3y" |
//! "5y") at insert time, then maintain four partial TTL indexes — one per
//! bucket — that age documents out at the matching rate.
//!
//! Retention is frozen at insert: a tier upgrade doesn't extend old events,
//! and a downgrade doesn't shrink them (which is intentional — no silent
//! data loss when a customer cancels or downgrades through Stripe).

use mongodb::bson::doc;
use mongodb::options::IndexOptions;
use mongodb::{Collection, IndexModel};
use std::time::Duration;

/// The four canonical retention buckets + their `expireAfterSeconds`. Must
/// match `PlanLimits::retention_bucket` values in `limits.rs`.
const BUCKETS: &[(&str, u64)] = &[
    ("30d", 30 * 24 * 3600),
    ("1y", 365 * 24 * 3600),
    ("3y", 3 * 365 * 24 * 3600),
    ("5y", 5 * 365 * 24 * 3600),
];

/// Ensure the four partial TTL indexes exist on the given time-series
/// collection. `time_field` is the collection's timeField (e.g. `clicked_at`
/// for click_events, `occurred_at` for conversion_events). `meta_field` is
/// the metaField name (always `"meta"` in this codebase).
///
/// Idempotent — MongoDB treats a createIndex with the same name + spec as a
/// no-op. Rename suffix is the bucket name so all four live side-by-side.
pub async fn ensure_retention_ttl_indexes<T>(
    collection: &Collection<T>,
    time_field: &str,
    meta_field: &str,
) where
    T: Send + Sync,
{
    for (bucket, expire_secs) in BUCKETS {
        let index_name = format!("retention_{bucket}");
        let partial_filter = doc! { format!("{meta_field}.retention_bucket"): *bucket };
        let opts = IndexOptions::builder()
            .name(index_name.clone())
            .expire_after(Duration::from_secs(*expire_secs))
            .partial_filter_expression(partial_filter)
            .build();
        let model = IndexModel::builder()
            .keys(doc! { time_field: 1 })
            .options(opts)
            .build();
        if let Err(e) = collection.create_index(model).await {
            let msg = e.to_string();
            // "already exists" / "IndexOptionsConflict" are both possible on
            // a subsequent startup depending on MongoDB version. Don't spam
            // error on those — log at info and continue.
            if msg.contains("already exists") || msg.contains("IndexOptionsConflict") {
                tracing::info!(index = index_name, "retention_ttl_index_already_present");
            } else {
                tracing::error!(
                    index = index_name,
                    error = %e,
                    "retention_ttl_index_create_failed"
                );
            }
        }
    }
}
