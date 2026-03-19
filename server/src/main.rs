mod api;
mod core;
mod error;

use std::sync::Arc;

use tower_http::cors::CorsLayer;
use tower_http::limit::RequestBodyLimitLayer;
use tracing_subscriber::prelude::*;
use tracing_subscriber::EnvFilter;

use x402_chain_eip155::{KnownNetworkEip155, V1Eip155Exact};
use x402_types::networks::USDC;

use crate::api::auth::repo::AuthRepo;
use crate::api::domains::repo::DomainsRepo;
use crate::api::links::repo::LinksRepo;
use crate::api::AppState;
use crate::core::config::Config;

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv();
    let cfg = Config::from_env();

    // Initialise Sentry (no-op if SENTRY_DSN is empty).
    let _sentry_guard = sentry::init(sentry::ClientOptions {
        dsn: cfg.sentry_dsn.parse().ok(),
        traces_sample_rate: 0.2,
        ..sentry::ClientOptions::default()
    });

    // Initialise tracing with Sentry layer (respects RUST_LOG env var).
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer())
        .with(sentry_tracing::layer())
        .init();

    // Connect to MongoDB (optional — server boots without it).
    let (auth_repo, links_repo, domains_repo) = if cfg.mongo_uri.is_empty() {
        tracing::warn!("MONGO_URI not set — auth, links, and domains disabled");
        (None, None, None)
    } else {
        match core::db::connect(&cfg.mongo_uri, &cfg.mongo_db).await {
            Some(database) => {
                tracing::info!(uri = %cfg.mongo_uri, db = %cfg.mongo_db, "Connected to MongoDB");
                let auth: Arc<dyn crate::api::auth::repo::AuthRepository> =
                    Arc::new(AuthRepo::new(&database).await);
                let links: Arc<dyn crate::api::links::repo::LinksRepository> =
                    Arc::new(LinksRepo::new(&database).await);
                let domains: Arc<dyn crate::api::domains::repo::DomainsRepository> =
                    Arc::new(DomainsRepo::new(&database).await);
                (Some(auth), Some(links), Some(domains))
            }
            None => {
                tracing::warn!("Failed to connect to MongoDB — auth, links, and domains disabled");
                (None, None, None)
            }
        }
    };

    // Initialise x402 payment facilitator (optional).
    let (facilitator, x402_price_tags) = if cfg.x402_enabled && !cfg.x402_recipient.is_empty() {
        let fc = core::cdp::CdpFacilitator::new(
            &cfg.cdp_api_key_id,
            &cfg.cdp_api_key_secret,
            &cfg.x402_facilitator_url,
        );
        let addr: x402_chain_eip155::chain::ChecksummedAddress = cfg
            .x402_recipient
            .parse()
            .expect("Invalid X402_RECIPIENT address");
        let usdc = USDC::base();
        let amount = usdc
            .parse(cfg.x402_price_display.as_str())
            .expect("Invalid X402_PRICE");
        let price_tag = V1Eip155Exact::price_tag(addr, amount);
        tracing::info!(
            facilitator = %cfg.x402_facilitator_url,
            recipient = %cfg.x402_recipient,
            price = %cfg.x402_price_display,
            "x402 payments enabled"
        );
        (Some(fc), vec![price_tag])
    } else {
        tracing::info!("x402 payments disabled");
        (None, vec![])
    };

    let state = Arc::new(AppState {
        auth_repo,
        links_repo,
        domains_repo,
        config: cfg.clone(),
        facilitator,
        x402_price_tags,
    });

    let app = api::router(state.clone())
        .with_state(state)
        .layer(RequestBodyLimitLayer::new(10 * 1024 * 1024))
        .layer(CorsLayer::permissive());

    let addr = cfg.bind_addr();
    tracing::info!("Starting Relay on {addr}");

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind to address");

    axum::serve(listener, app).await.expect("Server error");
}
