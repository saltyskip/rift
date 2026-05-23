//! m005_attribution_cutover — promote `installs.user_id` bindings to
//! `app_users` rows, then drop the legacy `installs` collection.
//!
//! Tiny on purpose. The new attribution model uses `app_users` for
//! identity, `install_events` for lifecycle history, and
//! `attribution_events` (with `user_id` stamped at write time) for the
//! event log. None of those need backfill from legacy `installs` rows
//! — except for one thing that genuinely matters: the **identity
//! bindings**.
//!
//! Without this migration, every pre-existing identified user would be
//! treated as unknown until they hit `/identify` again. Conversions
//! arriving from a backend integration (Stripe webhooks, scheduled
//! jobs) during that gap would be dropped as `unattributed`. So we
//! migrate just the (user_id, install_id) pairs and accept the analytics
//! loss on historical events.
//!
//! Three things this migration does NOT do (intentionally):
//!
//! 1. **No `install_events` backfill.** Historical install lifecycle is
//!    lost (funnel undercounts `acquired` etc. for pre-deploy installs).
//!    Acceptable analytics loss.
//! 2. **No `attribution_events.user_id` backfill.** Historical anonymous
//!    events stay anonymous. User-scoped reads over pre-deploy data
//!    return less. Acceptable analytics loss.
//! 3. **No reverse-lookup safety for `installs` data still being read.**
//!    The runtime no longer reads `installs` (Phase 6 dropped that
//!    code path); dropping the collection is safe.
//!
//! Idempotent on re-run: `app_users` upserts use `$addToSet` for the
//! install_ids array; `installs.drop()` is a no-op if the collection
//! is already gone.

use async_trait::async_trait;
use futures::TryStreamExt;
use mongodb::bson::{doc, oid::ObjectId, DateTime, Document};
use mongodb::Database;

crate::impl_container!(M005AttributionCutover);
pub struct M005AttributionCutover;

#[async_trait]
impl super::Migration for M005AttributionCutover {
    fn id(&self) -> &'static str {
        "m005_attribution_cutover"
    }

    fn description(&self) -> &'static str {
        "Migrate (user_id, install_id) bindings from installs to app_users, then drop installs"
    }

    async fn run(&self, db: &Database, dry_run: bool) -> Result<(), String> {
        let installs = db.collection::<Document>("installs");
        let app_users = db.collection::<Document>("app_users");

        let mut cursor = installs
            .find(doc! { "user_id": { "$ne": null } })
            .await
            .map_err(|e| format!("scan installs: {e}"))?;

        let mut stats = Stats::default();

        while let Some(install) = cursor
            .try_next()
            .await
            .map_err(|e| format!("cursor: {e}"))?
        {
            stats.installs_scanned += 1;

            let tenant_id = match install.get_object_id("tenant_id") {
                Ok(t) => t,
                Err(_) => {
                    stats.skipped += 1;
                    continue;
                }
            };
            let install_id = match install.get_str("install_id") {
                Ok(s) => s.to_string(),
                Err(_) => {
                    stats.skipped += 1;
                    continue;
                }
            };
            let user_id = match install.get_str("user_id") {
                Ok(s) => s.to_string(),
                Err(_) => {
                    stats.skipped += 1;
                    continue;
                }
            };
            let identified_at = install
                .get_datetime("identified_at")
                .or_else(|_| install.get_datetime("first_attributed_at"))
                .copied()
                .unwrap_or_else(|_| DateTime::now());

            if dry_run {
                stats.would_upsert += 1;
                continue;
            }

            app_users
                .update_one(
                    doc! { "tenant_id": &tenant_id, "user_id": &user_id },
                    doc! {
                        "$setOnInsert": {
                            "_id": ObjectId::new(),
                            "tenant_id": tenant_id,
                            "user_id": &user_id,
                            "identified_at": identified_at,
                        },
                        "$addToSet": { "install_ids": &install_id },
                        "$set": { "last_seen_at": identified_at },
                    },
                )
                .upsert(true)
                .await
                .map_err(|e| format!("upsert app_users: {e}"))?;
            stats.upserted += 1;
        }

        if dry_run {
            println!("  installs scanned: {}", stats.installs_scanned);
            println!("  skipped (malformed): {}", stats.skipped);
            println!("  Would upsert {} app_users row(s)", stats.would_upsert);
            println!("  Would drop the `installs` collection");
            return Ok(());
        }

        println!("  installs scanned: {}", stats.installs_scanned);
        println!("  skipped (malformed): {}", stats.skipped);
        println!("  Upserted {} app_users row(s)", stats.upserted);

        // Now drop the legacy installs collection. Idempotent — Mongo
        // returns success even if the collection doesn't exist.
        installs
            .drop()
            .await
            .map_err(|e| format!("drop installs: {e}"))?;
        println!("  Dropped legacy `installs` collection");

        Ok(())
    }
}

#[derive(Default)]
struct Stats {
    installs_scanned: u64,
    skipped: u64,
    would_upsert: u64,
    upserted: u64,
}
