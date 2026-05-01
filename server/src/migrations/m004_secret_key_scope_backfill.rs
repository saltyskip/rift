//! m004_secret_key_scope_backfill — set `scope: { type: "full" }` on every
//! `secret_keys` row that doesn't have a `scope` field yet.
//!
//! Background: we're flipping secret-key auth from "default-allow with
//! optional scope" to "scope required, default-deny." Every existing key was
//! issued with full tenant access, so we backfill to `KeyScope::Full`.
//!
//! Idempotent — the filter `scope: { $exists: false }` only matches
//! pre-migration rows. Re-running is safe (zero matches on the second run).
//!
//! Deployment order: apply this migration first, then deploy the v1 code
//! that grandfathers `None` to `Full`. A follow-up PR will remove the
//! grandfather path and require `scope` on every key.

use async_trait::async_trait;
use mongodb::bson::{doc, Document};
use mongodb::Database;

crate::impl_container!(M004SecretKeyScopeBackfill);
pub struct M004SecretKeyScopeBackfill;

#[async_trait]
impl super::Migration for M004SecretKeyScopeBackfill {
    fn id(&self) -> &'static str {
        "m004_secret_key_scope_backfill"
    }

    fn description(&self) -> &'static str {
        "Backfill SecretKeyDoc.scope = { type: \"full\" } on rows missing scope"
    }

    async fn run(&self, db: &Database, dry_run: bool) -> Result<(), String> {
        let secret_keys = db.collection::<Document>("secret_keys");
        let filter = doc! { "scope": { "$exists": false } };

        let to_backfill = secret_keys
            .count_documents(filter.clone())
            .await
            .map_err(|e| format!("count secret_keys without scope: {e}"))?;

        if dry_run {
            println!("  Would set scope: {{ type: \"full\" }} on {to_backfill} secret key(s)");
            return Ok(());
        }

        if to_backfill == 0 {
            println!("  No secret keys missing scope — nothing to do");
            return Ok(());
        }

        let result = secret_keys
            .update_many(filter, doc! { "$set": { "scope": { "type": "full" } } })
            .await
            .map_err(|e| format!("backfill scope: {e}"))?;

        println!(
            "  Backfilled scope on {} secret key(s) (matched {})",
            result.modified_count, result.matched_count
        );

        Ok(())
    }
}
