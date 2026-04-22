pub mod mocks;

use mongodb::bson::oid::ObjectId;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tokio::net::TcpListener;

use rift::app::AppState;
use rift::core::config::Config;
use rift::core::webhook_dispatcher::WebhookDispatcher;
use rift::services::auth::publishable_keys::repo::SdkKeysRepository;
use rift::services::auth::secret_keys::repo::{SecretKeyDoc, SecretKeysRepository};
use rift::services::auth::tenants::repo::{TenantDoc, TenantsRepository};
use rift::services::auth::usage::repo::UsageRepository;
use rift::services::auth::users::repo::UsersRepository;
use rift::services::domains::repo::DomainsRepository;

use mocks::{
    MockAppsRepo, MockConversionsRepo, MockDomainsRepo, MockLinksRepo, MockSdkKeysRepo,
    MockSecretKeysRepo, MockTenantsRepo, MockUsageRepo, MockUsersRepo, MockWebhookDispatcher,
    MockWebhooksRepo,
};

#[allow(dead_code)]
pub struct TestApp {
    pub addr: String,
    pub client: reqwest::Client,
    pub tenants_repo: Arc<MockTenantsRepo>,
    pub users_repo: Arc<MockUsersRepo>,
    pub secret_keys_repo: Arc<MockSecretKeysRepo>,
    pub usage_repo: Arc<MockUsageRepo>,
    pub links_repo: Arc<MockLinksRepo>,
    pub domains_repo: Arc<MockDomainsRepo>,
    pub apps_repo: Arc<MockAppsRepo>,
    pub webhooks_repo: Arc<MockWebhooksRepo>,
    pub webhook_dispatcher: Arc<MockWebhookDispatcher>,
    pub sdk_keys_repo: Arc<MockSdkKeysRepo>,
    pub threat_feed: rift::core::threat_feed::ThreatFeed,
}

impl TestApp {
    pub fn url(&self, path: &str) -> String {
        format!("http://{}{}", self.addr, path)
    }
}

pub async fn spawn_app() -> TestApp {
    let tenants_repo = Arc::new(MockTenantsRepo::default());
    let users_repo = Arc::new(MockUsersRepo::default());
    let secret_keys_repo = Arc::new(MockSecretKeysRepo::default());
    let usage_repo = Arc::new(MockUsageRepo::default());
    let links_repo = Arc::new(MockLinksRepo::default());
    let domains_repo = Arc::new(MockDomainsRepo::default());
    let apps_repo = Arc::new(MockAppsRepo::default());
    let webhooks_repo = Arc::new(MockWebhooksRepo::default());
    let webhook_dispatcher = Arc::new(MockWebhookDispatcher::default());
    let sdk_keys_repo = Arc::new(MockSdkKeysRepo::default());

    let config = Config {
        host: "127.0.0.1".to_string(),
        port: 0,
        mongo_uri: String::new(),
        mongo_db: String::new(),
        resend_api_key: String::new(),
        resend_from_email: String::new(),
        public_url: "http://localhost:0".to_string(),
        free_daily_limit: 5,
        sentry_dsn: String::new(),
        x402_facilitator_url: String::new(),
        x402_recipient: String::new(),
        x402_price_display: "0.01".to_string(),
        x402_enabled: false,
        cdp_api_key_id: String::new(),
        cdp_api_key_secret: String::new(),
        x402_description: String::new(),
        primary_domain: "riftl.ink".to_string(),
        fly_api_token: String::new(),
        fly_app_name: String::new(),
    };

    let threat_feed = rift::core::threat_feed::ThreatFeed::new();

    let links_service = Some(Arc::new(rift::services::links::service::LinksService::new(
        links_repo.clone() as Arc<dyn rift::services::links::repo::LinksRepository>,
        Some(domains_repo.clone() as Arc<dyn rift::services::domains::repo::DomainsRepository>),
        threat_feed.clone(),
        config.public_url.clone(),
    )));

    let conversions_repo: Arc<dyn rift::services::conversions::repo::ConversionsRepository> =
        Arc::new(MockConversionsRepo::default());
    let conversions_service = Some(Arc::new(
        rift::services::conversions::service::ConversionsService::new(
            conversions_repo.clone(),
            links_repo.clone() as Arc<dyn rift::services::links::repo::LinksRepository>,
            Some(webhook_dispatcher.clone() as Arc<dyn WebhookDispatcher>),
        ),
    ));

    let tenants_service = Arc::new(rift::services::auth::tenants::service::TenantsService::new(
        tenants_repo.clone() as Arc<dyn TenantsRepository>,
    ));

    let state = Arc::new(AppState {
        secret_keys_repo: Some(secret_keys_repo.clone() as Arc<dyn SecretKeysRepository>),
        usage_repo: Some(usage_repo.clone() as Arc<dyn UsageRepository>),
        links_repo: Some(
            links_repo.clone() as Arc<dyn rift::services::links::repo::LinksRepository>
        ),
        domains_repo: Some(
            domains_repo.clone() as Arc<dyn rift::services::domains::repo::DomainsRepository>
        ),
        apps_repo: Some(apps_repo.clone() as Arc<dyn rift::services::apps::repo::AppsRepository>),
        config,
        facilitator: None,
        x402_price_tags: vec![],
        webhooks_repo: Some(
            webhooks_repo.clone() as Arc<dyn rift::services::webhooks::repo::WebhooksRepository>
        ),
        webhook_dispatcher: Some(webhook_dispatcher.clone() as Arc<dyn WebhookDispatcher>),
        sdk_keys_repo: Some(sdk_keys_repo.clone()
            as Arc<dyn rift::services::auth::publishable_keys::repo::SdkKeysRepository>),
        links_service,
        users_service: Some(Arc::new(
            rift::services::auth::users::service::UsersService::new(
                tenants_service.clone(),
                users_repo.clone() as Arc<dyn UsersRepository>,
                secret_keys_repo.clone() as Arc<dyn SecretKeysRepository>,
            ),
        )),
        secret_keys_service: Some(Arc::new(
            rift::services::auth::secret_keys::service::SecretKeysService::new(
                secret_keys_repo.clone() as Arc<dyn SecretKeysRepository>,
                users_repo.clone() as Arc<dyn UsersRepository>,
            ),
        )),
        conversions_repo: Some(conversions_repo),
        conversions_service,
    });

    let app = rift::api::router(state.clone())
        .with_state(state)
        .into_make_service();

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap().to_string();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    TestApp {
        addr,
        client: reqwest::Client::new(),
        tenants_repo,
        users_repo,
        secret_keys_repo,
        usage_repo,
        threat_feed,
        links_repo,
        domains_repo,
        apps_repo,
        webhooks_repo,
        webhook_dispatcher,
        sdk_keys_repo,
    }
}

/// Seed a verified custom domain for a tenant.
pub async fn seed_verified_domain(app: &TestApp, tenant_id: &ObjectId, domain: &str) {
    app.domains_repo
        .create_domain(
            *tenant_id,
            domain.to_string(),
            "tok".to_string(),
            rift::services::domains::models::DomainRole::Primary,
        )
        .await
        .unwrap();
    app.domains_repo.mark_verified(domain).await.unwrap();
}

/// Seed an SDK key for a tenant and domain. Returns the raw pk_live_ key.
pub async fn seed_sdk_key(app: &TestApp, tenant_id: &ObjectId, domain: &str) -> String {
    let raw_key = format!("pk_live_test_{}", hex::encode(ObjectId::new().bytes()));
    let hash = hex::encode(sha2::Sha256::digest(raw_key.as_bytes()));
    let doc = rift::services::auth::publishable_keys::models::SdkKeyDoc {
        id: ObjectId::new(),
        tenant_id: *tenant_id,
        key_hash: hash,
        key_prefix: format!("{}...", &raw_key[..20]),
        domain: domain.to_string(),
        revoked: false,
        created_at: mongodb::bson::DateTime::now(),
    };
    app.sdk_keys_repo.create_key(&doc).await.unwrap();
    raw_key
}

/// Seed a verified API key and return (raw_key, tenant_id).
pub async fn seed_api_key(app: &TestApp) -> (String, ObjectId) {
    seed_api_key_with(app, "rl_live_test1234567890abcdef1234567890abcdef12345678").await
}

/// Seed a verified API key with a specific raw key and return (raw_key, tenant_id).
pub async fn seed_api_key_with(app: &TestApp, raw_key: &str) -> (String, ObjectId) {
    let hash = hex::encode(Sha256::digest(raw_key.as_bytes()));
    let tenant_id = ObjectId::new();
    let user_id = ObjectId::new();

    // Create tenant
    let tenant_doc = TenantDoc {
        id: Some(tenant_id),
        monthly_quota: 1000,
        ..TenantDoc::default()
    };
    app.tenants_repo.create(&tenant_doc).await.unwrap();

    // Create secret key
    let key_doc = SecretKeyDoc {
        id: ObjectId::new(),
        tenant_id,
        created_by: user_id,
        key_hash: hash,
        key_prefix: format!("{}...", &raw_key[..18]),
        created_at: mongodb::bson::DateTime::now(),
    };
    app.secret_keys_repo.create_key(&key_doc).await.unwrap();

    (raw_key.to_string(), tenant_id)
}
