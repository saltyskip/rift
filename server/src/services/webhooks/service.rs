//! Thin orchestrator for webhook lifecycle + quota enforcement.
//!
//! Same rule as DomainsService: the service layer is the one place both
//! `api/` and (future) `mcp/` consumers call, so quota lives here.

use rift_macros::requires;
use std::sync::Arc;

use super::models::{Webhook, WebhookError, WebhookEventType, WebhookFilters};
use super::repo::WebhooksRepository;
use crate::services::affiliates::repo::AffiliatesRepository;
use crate::services::auth::permissions::{AuthContext, Permission};
use crate::services::billing::quota::{QuotaChecker, Resource};

crate::impl_container!(WebhooksService);
pub struct WebhooksService {
    repo: Arc<dyn WebhooksRepository>,
    /// Used to validate `filters.affiliate_id` exists at create time.
    /// Optional so reduced-feature / test boots without an affiliates
    /// collection still construct — a `None` repo rejects any explicit
    /// affiliate filter with `AffiliateNotFound`.
    affiliates_repo: Option<Arc<dyn AffiliatesRepository>>,
    quota: Option<Arc<dyn QuotaChecker>>,
}

impl WebhooksService {
    pub fn new(
        repo: Arc<dyn WebhooksRepository>,
        affiliates_repo: Option<Arc<dyn AffiliatesRepository>>,
        quota: Option<Arc<dyn QuotaChecker>>,
    ) -> Self {
        Self {
            repo,
            affiliates_repo,
            quota,
        }
    }

    /// Create a webhook for the caller's tenant. Enforces the tier's webhook
    /// count quota and validates any affiliate filter before calling the repo.
    ///
    /// The id, signing secret, and `created_at` are generated here (not
    /// passed by the transport) — they're domain concerns, and the
    /// plaintext secret is returned once on the resulting [`Webhook`] for
    /// the caller to surface to the user.
    #[requires(Permission::WebhooksWrite)]
    pub async fn create_webhook(
        &self,
        ctx: &AuthContext,
        url: String,
        events: Vec<WebhookEventType>,
        filters: WebhookFilters,
    ) -> Result<Webhook, WebhookError> {
        if let Some(q) = &self.quota {
            q.check(&ctx.tenant_id, Resource::CreateWebhook).await?;
        }

        // Validate an affiliate filter points at a real affiliate so a
        // typo fails fast (400) rather than creating a webhook that
        // silently never matches. Webhook management is tenant-scoped
        // (affiliate credentials lack webhooks:write), so there's no
        // credential-scope pinning to resolve here — just existence.
        if let Some(affiliate_id) = filters.affiliate_id {
            let repo = self
                .affiliates_repo
                .as_ref()
                .ok_or(WebhookError::AffiliateNotFound)?;
            repo.get_by_id(&ctx.tenant_id, &affiliate_id)
                .await
                .map_err(WebhookError::Internal)?
                .ok_or(WebhookError::AffiliateNotFound)?;
        }

        let webhook = Webhook {
            id: crate::core::public_id::WebhookId::new(),
            tenant_id: ctx.tenant_id,
            url,
            secret: generate_secret(),
            events,
            filters,
            active: true,
            created_at: mongodb::bson::DateTime::now(),
        };

        self.repo
            .create_webhook(webhook.clone())
            .await
            .map_err(WebhookError::Internal)?;
        Ok(webhook)
    }
}

// ── Helpers ──

/// 32 random bytes, hex-encoded — the HMAC-SHA256 signing secret returned
/// to the caller once at creation.
fn generate_secret() -> String {
    use rand::Rng;
    let mut bytes = [0u8; 32];
    rand::rng().fill(&mut bytes);
    hex::encode(bytes)
}
