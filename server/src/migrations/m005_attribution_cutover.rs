//! m005_attribution_cutover — build the new attribution surface from
//! existing `installs` data.
//!
//! For every legacy `installs` row, this migration backfills:
//!
//! 1. `app_users` — one record per `(tenant_id, user_id)` with the install's
//!    install_id added to `install_ids`. Multi-install users (same user_id
//!    across rows) get all their install_ids merged into one record via
//!    `$addToSet`.
//! 2. `install_events` — one `install.created` event per install (using the
//!    install's `first_attributed_at` as the timestamp), plus an
//!    `install.identified` event per install whose user_id was bound.
//! 3. `attribution_events.user_id` — for every install with a user_id,
//!    backfills user_id onto prior anonymous attribution_events for that
//!    install. Mirrors the runtime identify-time backfill — same "only
//!    fill NULLs, never overwrite" rule.
//!
//! Idempotent on re-run: app_users upsert with `$addToSet` won't duplicate
//! install_ids; install_events writes are guarded by a "does install.created
//! already exist?" check; the attribution_events backfill only touches
//! `user_id: null` rows.
//!
//! Safe to run with the new v1 code already deployed: the runtime writes
//! new app_users / install_events as new installs come in; this migration
//! catches up the historical rows.

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
        "Backfill app_users, install_events, and attribution_events.user_id from legacy installs"
    }

    async fn run(&self, db: &Database, dry_run: bool) -> Result<(), String> {
        let installs = db.collection::<Document>("installs");
        let app_users = db.collection::<Document>("app_users");
        let install_events = db.collection::<Document>("install_events");
        let attribution_events = db.collection::<Document>("attribution_events");

        let mut cursor = installs
            .find(doc! {})
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
                    stats.skipped_no_tenant += 1;
                    continue;
                }
            };
            let install_id = match install.get_str("install_id") {
                Ok(s) => s.to_string(),
                Err(_) => {
                    stats.skipped_no_install_id += 1;
                    continue;
                }
            };
            let user_id = install.get_str("user_id").ok().map(|s| s.to_string());
            let first_attributed_at = install
                .get_datetime("first_attributed_at")
                .copied()
                .unwrap_or_else(|_| DateTime::now());
            let first_app_version = install
                .get_str("first_app_version")
                .ok()
                .map(|s| s.to_string());

            if dry_run {
                stats.would_create_install_created += 1;
                if user_id.is_some() {
                    stats.would_upsert_app_user += 1;
                    stats.would_create_install_identified += 1;
                }
                continue;
            }

            // 1. install.created — only if not already present.
            let already_created = install_events
                .find_one(doc! {
                    "tenant_id": &tenant_id,
                    "install_id": &install_id,
                    "event_type": "created",
                })
                .await
                .map_err(|e| format!("lookup install.created: {e}"))?
                .is_some();

            if !already_created {
                let mut created_doc = doc! {
                    "_id": ObjectId::new(),
                    "tenant_id": tenant_id,
                    "install_id": &install_id,
                    "event_type": "created",
                    "timestamp": first_attributed_at,
                };
                if let Some(v) = first_app_version.as_ref() {
                    created_doc.insert("app_version", v);
                }
                install_events
                    .insert_one(&created_doc)
                    .await
                    .map_err(|e| format!("insert install.created: {e}"))?;
                stats.install_created_inserted += 1;
            }

            // 2. app_users upsert + install.identified — only if bound.
            if let Some(user_id) = user_id.as_ref() {
                let identified_at = install
                    .get_datetime("identified_at")
                    .copied()
                    .unwrap_or(first_attributed_at);

                app_users
                    .update_one(
                        doc! { "tenant_id": &tenant_id, "user_id": user_id },
                        doc! {
                            "$setOnInsert": {
                                "_id": ObjectId::new(),
                                "tenant_id": tenant_id,
                                "user_id": user_id,
                                "identified_at": identified_at,
                            },
                            "$addToSet": { "install_ids": &install_id },
                            "$set": { "last_seen_at": identified_at },
                        },
                    )
                    .upsert(true)
                    .await
                    .map_err(|e| format!("upsert app_users: {e}"))?;
                stats.app_users_upserted += 1;

                let already_identified = install_events
                    .find_one(doc! {
                        "tenant_id": &tenant_id,
                        "install_id": &install_id,
                        "event_type": "identified",
                    })
                    .await
                    .map_err(|e| format!("lookup install.identified: {e}"))?
                    .is_some();
                if !already_identified {
                    install_events
                        .insert_one(&doc! {
                            "_id": ObjectId::new(),
                            "tenant_id": tenant_id,
                            "install_id": &install_id,
                            "event_type": "identified",
                            "timestamp": identified_at,
                            "user_id": user_id,
                        })
                        .await
                        .map_err(|e| format!("insert install.identified: {e}"))?;
                    stats.install_identified_inserted += 1;
                }

                // 3. attribution_events.user_id backfill — only NULLs.
                let backfilled = attribution_events
                    .update_many(
                        doc! {
                            "meta.tenant_id": tenant_id,
                            "meta.install_id": &install_id,
                            "user_id": { "$eq": null },
                        },
                        doc! { "$set": { "user_id": user_id } },
                    )
                    .await
                    .map_err(|e| format!("backfill attribution_events.user_id: {e}"))?;
                stats.attribution_events_backfilled += backfilled.modified_count;
            }
        }

        stats.print(dry_run);
        Ok(())
    }
}

#[derive(Default)]
struct Stats {
    installs_scanned: u64,
    skipped_no_tenant: u64,
    skipped_no_install_id: u64,

    would_create_install_created: u64,
    would_create_install_identified: u64,
    would_upsert_app_user: u64,

    install_created_inserted: u64,
    install_identified_inserted: u64,
    app_users_upserted: u64,
    attribution_events_backfilled: u64,
}

impl Stats {
    fn print(&self, dry_run: bool) {
        println!("  installs scanned: {}", self.installs_scanned);
        if self.skipped_no_tenant > 0 {
            println!("  skipped (no tenant_id): {}", self.skipped_no_tenant);
        }
        if self.skipped_no_install_id > 0 {
            println!("  skipped (no install_id): {}", self.skipped_no_install_id);
        }
        if dry_run {
            println!(
                "  Would create {} install.created event(s)",
                self.would_create_install_created
            );
            println!(
                "  Would create {} install.identified event(s)",
                self.would_create_install_identified
            );
            println!(
                "  Would upsert {} app_users row(s)",
                self.would_upsert_app_user
            );
            println!(
                "  (attribution_events backfill estimate skipped in dry-run — runs per install)"
            );
        } else {
            println!(
                "  Inserted {} install.created event(s)",
                self.install_created_inserted
            );
            println!(
                "  Inserted {} install.identified event(s)",
                self.install_identified_inserted
            );
            println!("  Upserted {} app_users row(s)", self.app_users_upserted);
            println!(
                "  Backfilled user_id on {} attribution_event row(s)",
                self.attribution_events_backfilled
            );
        }
    }
}
