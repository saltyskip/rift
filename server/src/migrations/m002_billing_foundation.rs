use async_trait::async_trait;
use mongodb::bson::{doc, Document};
use mongodb::Database;

/// Backfills billing fields on existing tenants so the server can read them
/// after the Plan A rollout. Every tenant ends up with:
///   plan_tier:        "free"
///   billing_method:   "free"
///   status:           "active"
///
/// The rest of the new `TenantDoc` fields (`current_period_*`, `stripe_*`,
/// `comp_*`) are optional and stay absent on existing rows.
///
/// The update is gated on `plan_tier` being missing — re-running this
/// migration is a no-op once applied. Serde `#[serde(default)]` on the new
/// fields lets the server read old rows even before the migration runs, so
/// there's no ordering constraint with a deploy.
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
            println!("  Would backfill {candidates} tenant(s) with plan_tier='free', billing_method='free', status='active'");
            return Ok(());
        }

        let update = doc! {
            "$set": {
                "plan_tier": "free",
                "billing_method": "free",
                "status": "active",
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
