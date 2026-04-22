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
mod tests {
    use super::*;
    use crate::services::auth::tenants::repo::TenantDoc;
    use mongodb::bson::DateTime;

    fn tenant(plan: PlanTier, comp: Option<PlanTier>, until: Option<DateTime>) -> TenantDoc {
        TenantDoc {
            plan_tier: plan,
            comp_tier: comp,
            comp_until: until,
            ..TenantDoc::default()
        }
    }

    fn date(ms: i64) -> DateTime {
        DateTime::from_millis(ms)
    }

    #[test]
    fn no_comp_returns_paid_plan() {
        assert_eq!(
            effective_tier(&tenant(PlanTier::Pro, None, None), date(0)),
            PlanTier::Pro
        );
    }

    #[test]
    fn comp_with_no_until_is_forever() {
        assert_eq!(
            effective_tier(
                &tenant(PlanTier::Free, Some(PlanTier::Business), None),
                date(10_000_000)
            ),
            PlanTier::Business
        );
    }

    #[test]
    fn comp_before_until_applies() {
        assert_eq!(
            effective_tier(
                &tenant(PlanTier::Free, Some(PlanTier::Scale), Some(date(2_000_000))),
                date(1_000_000)
            ),
            PlanTier::Scale
        );
    }

    #[test]
    fn comp_after_until_falls_back_to_paid() {
        assert_eq!(
            effective_tier(
                &tenant(
                    PlanTier::Pro,
                    Some(PlanTier::Business),
                    Some(date(1_000_000))
                ),
                date(2_000_000)
            ),
            PlanTier::Pro
        );
    }
}
