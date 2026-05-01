use async_trait::async_trait;
use mongodb::bson::{doc, Document};
use mongodb::Database;

crate::impl_container!(M002BillingFoundation);
/// Backfills billing fields on existing tenants so the server can read them
/// after the Plan A rollout. Every existing tenant ends up with:
///   plan_tier:      "free"   (the baseline, if comp ever expires)
///   billing_method: "free"
///   status:         "active"
///   comp_tier:      "scale"  (grandfather — unlimited access)
///   comp_until:     (unset → forever)
///
/// The comp overlay means every existing tenant has effective_tier=Scale
/// on day 1, so enforcement flipping on never surprises them. New tenants
/// created after the migration runs get the plain Free default from
/// TenantDoc::default() and go through normal Stripe/x402 paths.
///
/// The update is gated on `plan_tier` being missing — re-running is a no-op
/// once applied. Serde `#[serde(default)]` on the new fields lets the server
/// read old rows even before the migration runs, so there's no deploy-
/// ordering constraint.
pub struct M002BillingFoundation;

#[async_trait]
impl super::Migration for M002BillingFoundation {
    fn id(&self) -> &'static str {
        "m002_billing_foundation"
    }

    fn description(&self) -> &'static str {
        "Backfill plan_tier/billing_method/status on existing tenants"
    }

    async fn run(&self, db: &Database, dry_run: bool) -> Result<(), String> {
        let tenants = db.collection::<Document>("tenants");

        let filter = doc! { "plan_tier": { "$exists": false } };
        let candidates = tenants
            .count_documents(filter.clone())
            .await
            .map_err(|e| format!("Failed to count candidates: {e}"))?;

        if dry_run {
            println!(
                "  Would backfill {candidates} tenant(s) with plan_tier='free', billing_method='free', status='active', comp_tier='scale' (grandfathered — unlimited via comp overlay)"
            );
            return Ok(());
        }

        let update = doc! {
            "$set": {
                "plan_tier": "free",
                "billing_method": "free",
                "status": "active",
                // Grandfather: effective_tier = Scale via the comp overlay.
                // comp_until left unset = forever. An operator can revoke
                // this via a targeted mongosh update if a specific tenant
                // should be moved to paid billing.
                "comp_tier": "scale",
            }
        };

        let result = tenants
            .update_many(filter, update)
            .await
            .map_err(|e| format!("update_many failed: {e}"))?;

        println!(
            "  Backfilled {} tenant(s) (matched {})",
            result.modified_count, result.matched_count
        );
        Ok(())
    }
}
