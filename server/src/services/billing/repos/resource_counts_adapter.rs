//! Bridges existing `*Repository::count_by_tenant` methods into the single
//! `ResourceCounts` trait that `QuotaService` consumes. Keeps the quota layer
//! from taking a fan-out of repo dependencies directly.

use async_trait::async_trait;
use mongodb::bson::oid::ObjectId;
use std::sync::Arc;

use super::super::quota::{Resource, ResourceCounts};
use crate::services::auth::users::repo::UsersRepository;
use crate::services::domains::repo::DomainsRepository;
use crate::services::links::repo::LinksRepository;
use crate::services::webhooks::repo::WebhooksRepository;

pub struct RepoResourceCounts {
    pub links: Arc<dyn LinksRepository>,
    pub domains: Arc<dyn DomainsRepository>,
    pub users: Arc<dyn UsersRepository>,
    pub webhooks: Arc<dyn WebhooksRepository>,
}

#[async_trait]
impl ResourceCounts for RepoResourceCounts {
    async fn count(&self, tenant_id: &ObjectId, resource: Resource) -> Result<u64, String> {
        match resource {
            Resource::CreateLink => self.links.count_links_by_tenant(tenant_id).await,
            Resource::CreateDomain => self.domains.count_by_tenant(tenant_id).await,
            Resource::InviteTeamMember => self
                .users
                .count_verified_by_tenant(tenant_id)
                .await
                .map(|n| n as u64),
            Resource::CreateWebhook => self.webhooks.count_by_tenant(tenant_id).await,
            // TrackEvent uses the atomic counter path, not ResourceCounts.
            Resource::TrackEvent => Ok(0),
        }
    }
}
