pub mod mocks;

use mongodb::bson::oid::ObjectId;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tokio::net::TcpListener;

use relay::api::auth::repo::{ApiKeyDoc, AuthRepository};
use relay::api::AppState;
use relay::core::config::Config;

use mocks::{MockAuthRepo, MockDomainsRepo, MockLinksRepo};

#[allow(dead_code)]
pub struct TestApp {
    pub addr: String,
    pub client: reqwest::Client,
    pub auth_repo: Arc<MockAuthRepo>,
    pub links_repo: Arc<MockLinksRepo>,
    pub domains_repo: Arc<MockDomainsRepo>,
}

impl TestApp {
    pub fn url(&self, path: &str) -> String {
        format!("http://{}{}", self.addr, path)
    }
}

pub async fn spawn_app() -> TestApp {
    let auth_repo = Arc::new(MockAuthRepo::default());
    let links_repo = Arc::new(MockLinksRepo::default());
    let domains_repo = Arc::new(MockDomainsRepo::default());

    let config = Config {
        host: "127.0.0.1".to_string(),
        port: 0,
        mongo_uri: String::new(),
        mongo_db: String::new(),
        resend_api_key: String::new(),
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
    };

    let state = Arc::new(AppState {
        auth_repo: Some(auth_repo.clone() as Arc<dyn AuthRepository>),
        links_repo: Some(links_repo.clone() as Arc<dyn relay::api::links::repo::LinksRepository>),
        domains_repo: Some(
            domains_repo.clone() as Arc<dyn relay::api::domains::repo::DomainsRepository>
        ),
        config,
        facilitator: None,
        x402_price_tags: vec![],
    });

    let app = relay::api::router(state.clone())
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
        auth_repo,
        links_repo,
        domains_repo,
    }
}

/// Seed a verified API key and return (raw_key, ObjectId).
pub async fn seed_api_key(app: &TestApp) -> (String, ObjectId) {
    let raw_key = "rl_live_test1234567890abcdef1234567890abcdef12345678";
    let hash = hex::encode(Sha256::digest(raw_key.as_bytes()));
    let key_id = ObjectId::new();

    let doc = ApiKeyDoc {
        id: Some(key_id),
        email: format!("test-{}@example.com", key_id.to_hex()),
        key_hash: hash,
        key_prefix: "rl_live_test1234...".to_string(),
        verified: true,
        verify_token: None,
        monthly_quota: 1000,
        created_at: mongodb::bson::DateTime::now(),
    };

    app.auth_repo.upsert_key(&doc).await.unwrap();
    (raw_key.to_string(), key_id)
}
