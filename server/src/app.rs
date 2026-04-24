use std::sync::Arc;

use crate::core::cdp::CdpFacilitator;
use crate::core::config::Config;
use crate::core::webhook_dispatcher::WebhookDispatcher;
use crate::services::apps::repo::AppsRepository;
use crate::services::auth::publishable_keys::repo::SdkKeysRepository;
use crate::services::auth::secret_keys::repo::SecretKeysRepository;
use crate::services::auth::secret_keys::service::SecretKeysService;
use crate::services::auth::tenants::repo::TenantsRepository;
use crate::services::auth::usage::repo::UsageRepository;
use crate::services::auth::users::service::UsersService;
use crate::services::billing::repos::magic_links::MagicLinksRepository;
use crate::services::billing::repos::stripe_webhook_dedup::StripeWebhookDedupRepository;
use crate::services::billing::service::BillingService;
use crate::services::conversions::repo::ConversionsRepository;
use crate::services::conversions::service::ConversionsService;
use crate::services::domains::repo::DomainsRepository;
use crate::services::domains::service::DomainsService;
use crate::services::links::repo::LinksRepository;
use crate::services::links::service::LinksService;
use crate::services::webhooks::repo::WebhooksRepository;
use crate::services::webhooks::service::WebhooksService;

use x402_types::proto::v1;

/// Shared application state available to all route handlers.
pub struct AppState {
    pub tenants_repo: Option<Arc<dyn TenantsRepository>>,
    pub stripe_webhook_dedup: Option<Arc<dyn StripeWebhookDedupRepository>>,
    pub magic_links_repo: Option<Arc<dyn MagicLinksRepository>>,
    pub secret_keys_repo: Option<Arc<dyn SecretKeysRepository>>,
    pub usage_repo: Option<Arc<dyn UsageRepository>>,
    pub links_repo: Option<Arc<dyn LinksRepository>>,
    pub domains_repo: Option<Arc<dyn DomainsRepository>>,
    pub apps_repo: Option<Arc<dyn AppsRepository>>,
    pub config: Config,
    pub facilitator: Option<CdpFacilitator>,
    pub x402_price_tags: Vec<v1::PriceTag>,
    pub webhooks_repo: Option<Arc<dyn WebhooksRepository>>,
    pub webhook_dispatcher: Option<Arc<dyn WebhookDispatcher>>,
    pub sdk_keys_repo: Option<Arc<dyn SdkKeysRepository>>,
    pub conversions_repo: Option<Arc<dyn ConversionsRepository>>,
    pub links_service: Option<Arc<LinksService>>,
    pub domains_service: Option<Arc<DomainsService>>,
    pub webhooks_service: Option<Arc<WebhooksService>>,
    pub users_service: Option<Arc<UsersService>>,
    pub secret_keys_service: Option<Arc<SecretKeysService>>,
    pub conversions_service: Option<Arc<ConversionsService>>,
    pub billing_service: Option<Arc<BillingService>>,
}
