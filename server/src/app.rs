use std::sync::Arc;

use crate::core::cdp::CdpFacilitator;
use crate::core::config::Config;
use crate::core::webhook_dispatcher::WebhookDispatcher;
use crate::services::apps::repo::AppsRepository;
use crate::services::auth::publishable_keys::repo::SdkKeysRepository;
use crate::services::auth::secret_keys::new_repo::SecretKeysRepository;
use crate::services::auth::secret_keys::repo::AuthRepository;
use crate::services::auth::secret_keys::service::SecretKeysService;
use crate::services::auth::tenants::repo::TenantsRepository;
use crate::services::auth::usage::repo::UsageRepository;
use crate::services::auth::users::repo::UsersRepository;
use crate::services::auth::users::service::UsersService;
use crate::services::domains::repo::DomainsRepository;
use crate::services::links::repo::LinksRepository;
use crate::services::links::service::LinksService;
use crate::services::webhooks::repo::WebhooksRepository;

use x402_types::proto::v1;

/// Shared application state available to all route handlers.
///
/// New repo fields (tenants, users, secret_keys, usage) are wired up but not yet
/// read by any handler — that happens in the next PR. Allow dead_code for now.
#[allow(dead_code)]
pub struct AppState {
    pub auth_repo: Option<Arc<dyn AuthRepository>>,
    pub tenants_repo: Option<Arc<dyn TenantsRepository>>,
    pub users_repo: Option<Arc<dyn UsersRepository>>,
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
    pub links_service: Option<Arc<LinksService>>,
    pub users_service: Option<Arc<UsersService>>,
    pub secret_keys_service: Option<Arc<SecretKeysService>>,
}
