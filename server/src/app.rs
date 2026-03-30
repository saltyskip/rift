use std::sync::Arc;

use crate::core::cdp::CdpFacilitator;
use crate::core::config::Config;
use crate::core::webhook_dispatcher::WebhookDispatcher;
use crate::services::apps::repo::AppsRepository;
use crate::services::auth::publishable_keys::repo::SdkKeysRepository;
use crate::services::auth::secret_keys::repo::AuthRepository;
use crate::services::domains::repo::DomainsRepository;
use crate::services::links::repo::LinksRepository;
use crate::services::links::service::LinksService;
use crate::services::webhooks::repo::WebhooksRepository;

use x402_types::proto::v1;

/// Shared application state available to all route handlers.
pub struct AppState {
    pub auth_repo: Option<Arc<dyn AuthRepository>>,
    pub links_repo: Option<Arc<dyn LinksRepository>>,
    pub domains_repo: Option<Arc<dyn DomainsRepository>>,
    pub apps_repo: Option<Arc<dyn AppsRepository>>,
    pub config: Config,
    pub facilitator: Option<CdpFacilitator>,
    pub x402_price_tags: Vec<v1::PriceTag>,
    pub webhooks_repo: Option<Arc<dyn WebhooksRepository>>,
    pub webhook_dispatcher: Option<Arc<dyn WebhookDispatcher>>,
    pub sdk_keys_repo: Option<Arc<dyn SdkKeysRepository>>,
    pub links_service: Option<Arc<LinksService>>,
}
