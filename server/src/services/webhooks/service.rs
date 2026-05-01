//! Thin orchestrator for webhook lifecycle + quota enforcement.
//!
//! Same rule as DomainsService: the service layer is the one place both
//! `api/` and (future) `mcp/` consumers call, so quota lives here.

use mongodb::bson::oid::ObjectId;
use std::sync::Arc;

use super::models::{Webhook, WebhookError, WebhookEventType};
use super::repo::WebhooksRepository;
use crate::services::billing::quota::{QuotaChecker, Resource};

pub struct WebhooksService {
    repo: Arc<dyn WebhooksRepository>,
    quota: Option<Arc<dyn QuotaChecker>>,
}

impl WebhooksService {
    pub fn new(repo: Arc<dyn WebhooksRepository>, quota: Option<Arc<dyn QuotaChecker>>) -> Self {
        Self { repo, quota }
    }

    /// Create a webhook for a tenant. Enforces the tier's webhook count
    /// quota before calling the repo.
    pub async fn create_webhook(
        &self,
        tenant_id: ObjectId,
        id: ObjectId,
        url: String,
        secret: String,
        events: Vec<WebhookEventType>,
        created_at: mongodb::bson::DateTime,
    ) -> Result<Webhook, WebhookError> {
        if let Some(q) = &self.quota {
            q.check(&tenant_id, Resource::CreateWebhook).await?;
        }

        let webhook = Webhook {
            id,
            tenant_id,
            url,
            secret,
            events,
            active: true,
            created_at,
        };

        self.repo
            .create_webhook(webhook.clone())
            .await
            .map_err(WebhookError::Internal)?;
        Ok(webhook)
    }
}
