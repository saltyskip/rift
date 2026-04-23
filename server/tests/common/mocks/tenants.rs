use async_trait::async_trait;
use mongodb::bson::oid::ObjectId;
use std::sync::Mutex;

use rift::services::auth::tenants::repo::{
    BillingMethod, PlanTier, SubscriptionStatus, SubscriptionUpdate, TenantDoc, TenantsRepository,
};

#[derive(Default)]
pub struct MockTenantsRepo {
    pub tenants: Mutex<Vec<TenantDoc>>,
}

#[async_trait]
impl TenantsRepository for MockTenantsRepo {
    async fn create(&self, doc: &TenantDoc) -> Result<(), String> {
        self.tenants.lock().unwrap().push(doc.clone());
        Ok(())
    }

    async fn find_by_id(&self, id: &ObjectId) -> Result<Option<TenantDoc>, String> {
        Ok(self
            .tenants
            .lock()
            .unwrap()
            .iter()
            .find(|t| t.id.as_ref() == Some(id))
            .cloned())
    }

    async fn find_by_stripe_customer_id(
        &self,
        customer_id: &str,
    ) -> Result<Option<TenantDoc>, String> {
        Ok(self
            .tenants
            .lock()
            .unwrap()
            .iter()
            .find(|t| t.stripe_customer_id.as_deref() == Some(customer_id))
            .cloned())
    }

    async fn apply_subscription_update(
        &self,
        tenant_id: &ObjectId,
        update: SubscriptionUpdate,
    ) -> Result<bool, String> {
        let mut guard = self.tenants.lock().unwrap();
        if let Some(t) = guard.iter_mut().find(|t| t.id.as_ref() == Some(tenant_id)) {
            if let Some(x) = update.plan_tier {
                t.plan_tier = x;
            }
            if let Some(x) = update.billing_method {
                t.billing_method = x;
            }
            if let Some(x) = update.status {
                t.status = x;
            }
            if let Some(x) = update.current_period_start {
                t.current_period_start = Some(x);
            }
            if let Some(x) = update.current_period_end {
                t.current_period_end = Some(x);
            }
            if let Some(x) = update.stripe_customer_id {
                t.stripe_customer_id = Some(x);
            }
            if let Some(x) = update.stripe_subscription_id {
                t.stripe_subscription_id = Some(x);
            }
            Ok(true)
        } else {
            Ok(false)
        }
    }

    async fn clear_subscription(&self, tenant_id: &ObjectId) -> Result<bool, String> {
        let mut guard = self.tenants.lock().unwrap();
        if let Some(t) = guard.iter_mut().find(|t| t.id.as_ref() == Some(tenant_id)) {
            t.plan_tier = PlanTier::Free;
            t.billing_method = BillingMethod::Free;
            t.status = SubscriptionStatus::Canceled;
            t.current_period_start = None;
            t.current_period_end = None;
            t.stripe_subscription_id = None;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}
