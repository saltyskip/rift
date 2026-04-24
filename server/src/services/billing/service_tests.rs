use super::*;
use crate::services::auth::tenants::repo::{PlanTier, TenantDoc};
use mongodb::bson::DateTime;

use async_trait::async_trait;
use std::sync::Mutex;

#[derive(Default)]
struct MockRepo {
    tenants: Mutex<Vec<TenantDoc>>,
}

#[async_trait]
impl TenantsRepository for MockRepo {
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
        _customer_id: &str,
    ) -> Result<Option<TenantDoc>, String> {
        Ok(None)
    }

    async fn find_by_owner_email(&self, _email: &str) -> Result<Option<TenantDoc>, String> {
        Ok(None)
    }

    async fn apply_subscription_update(
        &self,
        _tenant_id: &ObjectId,
        _update: crate::services::auth::tenants::repo::SubscriptionUpdate,
    ) -> Result<bool, String> {
        Ok(true)
    }

    async fn clear_subscription(&self, _tenant_id: &ObjectId) -> Result<bool, String> {
        Ok(true)
    }
}

async fn setup(tenant: TenantDoc) -> (BillingService, ObjectId) {
    let repo = Arc::new(MockRepo::default());
    let id = tenant.id.expect("test tenant needs an id");
    repo.create(&tenant).await.unwrap();
    let svc = BillingService::new(repo as Arc<dyn TenantsRepository>);
    (svc, id)
}

#[tokio::test]
async fn status_reports_free_default() {
    let id = ObjectId::new();
    let (svc, id) = setup(TenantDoc {
        id: Some(id),
        ..TenantDoc::default()
    })
    .await;
    let s = svc.status(&id).await.unwrap();
    assert_eq!(s.plan_tier, PlanTier::Free);
    assert_eq!(s.effective_tier, PlanTier::Free);
    assert!(!s.comp_active);
}

#[tokio::test]
async fn status_reports_active_comp_as_effective_tier() {
    let id = ObjectId::new();
    let (svc, id) = setup(TenantDoc {
        id: Some(id),
        plan_tier: PlanTier::Free,
        comp_tier: Some(PlanTier::Business),
        comp_until: None,
        ..TenantDoc::default()
    })
    .await;
    let s = svc.status(&id).await.unwrap();
    assert_eq!(s.plan_tier, PlanTier::Free);
    assert_eq!(s.effective_tier, PlanTier::Business);
    assert!(s.comp_active);
}

#[tokio::test]
async fn status_treats_expired_comp_as_inactive() {
    let id = ObjectId::new();
    let (svc, id) = setup(TenantDoc {
        id: Some(id),
        plan_tier: PlanTier::Pro,
        comp_tier: Some(PlanTier::Scale),
        comp_until: Some(DateTime::from_millis(1)), // effectively past
        ..TenantDoc::default()
    })
    .await;
    let s = svc.status(&id).await.unwrap();
    assert_eq!(s.effective_tier, PlanTier::Pro);
    assert!(!s.comp_active);
}

#[tokio::test]
async fn status_missing_tenant_errors() {
    let repo = Arc::new(MockRepo::default());
    let svc = BillingService::new(repo as Arc<dyn TenantsRepository>);
    let err = svc.status(&ObjectId::new()).await.unwrap_err();
    assert!(matches!(err, BillingError::TenantNotFound));
}
