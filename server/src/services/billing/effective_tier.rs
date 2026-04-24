use mongodb::bson;

use crate::services::auth::tenants::repo::{PlanTier, TenantDoc};

/// Single source of truth for "what tier does this tenant's behavior reflect?".
///
/// An active comp override wins over the paid plan; otherwise returns the
/// paid `plan_tier`. Never touches Stripe / x402 state — comp is an orthogonal
/// overlay.
pub fn effective_tier(tenant: &TenantDoc, now: bson::DateTime) -> PlanTier {
    if let Some(comp) = tenant.comp_tier {
        let still_valid = tenant.comp_until.map(|until| now < until).unwrap_or(true);
        if still_valid {
            return comp;
        }
    }
    tenant.plan_tier
}

#[cfg(test)]
#[path = "effective_tier_tests.rs"]
mod tests;
