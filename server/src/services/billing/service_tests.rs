use super::*;
use crate::core::public_id::{SecretKeyId, TenantId};
use crate::services::auth::permissions::AuthContext;
use crate::services::auth::secret_keys::repo::KeyScope;
use crate::services::auth::tenants::repo::{PlanTier, TenantDoc};
use mongodb::bson::oid::ObjectId;
use mongodb::bson::DateTime;

use async_trait::async_trait;
use std::sync::Mutex;

fn full_ctx_for(tenant_id: TenantId) -> AuthContext {
    AuthContext::for_secret_key(tenant_id, SecretKeyId::new(), Some(&KeyScope::Full))
}

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
            .find(|t| t.id.map(|i| i.to_object_id()).as_ref() == Some(id))
            .cloned())
    }

    async fn find_by_stripe_customer_id(
        &self,
        _customer_id: &str,
    ) -> Result<Option<TenantDoc>, String> {
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

async fn setup(tenant: TenantDoc) -> (BillingService, TenantId) {
    let repo = Arc::new(MockRepo::default());
    let id = tenant.id.expect("test tenant needs an id");
    repo.create(&tenant).await.unwrap();
    let svc = BillingService::new(repo as Arc<dyn TenantsRepository>);
    (svc, id)
}

#[tokio::test]
async fn status_reports_free_default() {
    let id = TenantId::new();
    let (svc, id) = setup(TenantDoc {
        id: Some(id),
        ..TenantDoc::default()
    })
    .await;
    let s = svc.status(&full_ctx_for(id)).await.unwrap();
    assert_eq!(s.plan_tier, PlanTier::Free);
    assert_eq!(s.effective_tier, PlanTier::Free);
    assert!(!s.comp_active);
}

#[tokio::test]
async fn status_reports_active_comp_as_effective_tier() {
    let id = TenantId::new();
    let (svc, id) = setup(TenantDoc {
        id: Some(id),
        plan_tier: PlanTier::Free,
        comp_tier: Some(PlanTier::Business),
        comp_until: None,
        ..TenantDoc::default()
    })
    .await;
    let s = svc.status(&full_ctx_for(id)).await.unwrap();
    assert_eq!(s.plan_tier, PlanTier::Free);
    assert_eq!(s.effective_tier, PlanTier::Business);
    assert!(s.comp_active);
}

#[tokio::test]
async fn status_treats_expired_comp_as_inactive() {
    let id = TenantId::new();
    let (svc, id) = setup(TenantDoc {
        id: Some(id),
        plan_tier: PlanTier::Pro,
        comp_tier: Some(PlanTier::Scale),
        comp_until: Some(DateTime::from_millis(1)), // effectively past
        ..TenantDoc::default()
    })
    .await;
    let s = svc.status(&full_ctx_for(id)).await.unwrap();
    assert_eq!(s.effective_tier, PlanTier::Pro);
    assert!(!s.comp_active);
}

#[tokio::test]
async fn status_missing_tenant_errors() {
    let repo = Arc::new(MockRepo::default());
    let svc = BillingService::new(repo as Arc<dyn TenantsRepository>);
    let err = svc
        .status(&full_ctx_for(TenantId::new()))
        .await
        .unwrap_err();
    assert!(matches!(err, BillingError::TenantNotFound));
}
