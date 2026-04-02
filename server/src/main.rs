#[cfg(feature = "api")]
mod api;
mod app;
mod core;
mod error;
#[cfg(feature = "mcp")]
mod mcp;
mod services;

use std::sync::Arc;

use tower_http::cors::CorsLayer;
use tower_http::limit::RequestBodyLimitLayer;
use tracing_subscriber::prelude::*;
use tracing_subscriber::EnvFilter;

use x402_chain_eip155::{KnownNetworkEip155, V1Eip155Exact};
use x402_types::networks::USDC;

use crate::app::AppState;
use crate::core::config::Config;
use crate::services::apps::repo::AppsRepo;
use crate::services::auth::publishable_keys::repo::SdkKeysRepo;
use crate::services::auth::secret_keys::new_repo::SecretKeysRepo;
use crate::services::auth::secret_keys::repo::AuthRepo;
use crate::services::auth::tenants::repo::TenantsRepo;
use crate::services::auth::usage::repo::UsageRepo;
use crate::services::auth::users::repo::UsersRepo;
use crate::services::domains::repo::DomainsRepo;
use crate::services::links::repo::LinksRepo;
use crate::services::webhooks::dispatcher::RiftWebhookDispatcher;
use crate::services::webhooks::repo::WebhooksRepo;

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
    let (
        auth_repo,
        tenants_repo,
        users_repo,
        new_secret_keys_repo,
        usage_repo,
        links_repo,
        domains_repo,
        apps_repo,
        webhooks_repo,
        sdk_keys_repo,
    ) = if cfg.mongo_uri.is_empty() {
        tracing::warn!(
            "MONGO_URI not set — auth, links, domains, apps, webhooks, and sdk_keys disabled"
        );
        (None, None, None, None, None, None, None, None, None, None)
    } else {
        match core::db::connect(&cfg.mongo_uri, &cfg.mongo_db).await {
            Some(database) => {
                tracing::info!(uri = %cfg.mongo_uri, db = %cfg.mongo_db, "Connected to MongoDB");
                let auth: Arc<dyn crate::services::auth::secret_keys::repo::AuthRepository> =
                    Arc::new(AuthRepo::new(&database).await);
                let tenants: Arc<dyn crate::services::auth::tenants::repo::TenantsRepository> =
                    Arc::new(TenantsRepo::new(&database).await);
                let users: Arc<dyn crate::services::auth::users::repo::UsersRepository> =
                    Arc::new(UsersRepo::new(&database).await);
                let secret_keys: Arc<
                    dyn crate::services::auth::secret_keys::new_repo::SecretKeysRepository,
                > = Arc::new(SecretKeysRepo::new(&database).await);
                let usage: Arc<dyn crate::services::auth::usage::repo::UsageRepository> =
                    Arc::new(UsageRepo::new(&database).await);
                let links: Arc<dyn crate::services::links::repo::LinksRepository> =
                    Arc::new(LinksRepo::new(&database).await);
                let domains: Arc<dyn crate::services::domains::repo::DomainsRepository> =
                    Arc::new(DomainsRepo::new(&database).await);
                let apps: Arc<dyn crate::services::apps::repo::AppsRepository> =
                    Arc::new(AppsRepo::new(&database).await);
                let webhooks: Arc<dyn crate::services::webhooks::repo::WebhooksRepository> =
                    Arc::new(WebhooksRepo::new(&database).await);
                let sdk_keys: Arc<
                    dyn crate::services::auth::publishable_keys::repo::SdkKeysRepository,
                > = Arc::new(SdkKeysRepo::new(&database).await);
                (
                    Some(auth),
                    Some(tenants),
                    Some(users),
                    Some(secret_keys),
                    Some(usage),
                    Some(links),
                    Some(domains),
                    Some(apps),
                    Some(webhooks),
                    Some(sdk_keys),
                )
            }
            None => {
                tracing::warn!(
                    "Failed to connect to MongoDB — auth, links, domains, apps, webhooks, and sdk_keys disabled"
                );
                (None, None, None, None, None, None, None, None, None, None)
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

    // Initialize threat feed and start background refresh (every 30 minutes).
    let threat_feed = core::threat_feed::ThreatFeed::new();
    threat_feed.clone().start_background_refresh(30 * 60);

    let webhook_dispatcher: Option<Arc<dyn crate::core::webhook_dispatcher::WebhookDispatcher>> =
        webhooks_repo.as_ref().map(|repo| {
            Arc::new(RiftWebhookDispatcher::new(repo.clone()))
                as Arc<dyn crate::core::webhook_dispatcher::WebhookDispatcher>
        });

    let links_service = links_repo.as_ref().map(|repo| {
        Arc::new(crate::services::links::service::LinksService::new(
            repo.clone(),
            domains_repo.clone(),
            threat_feed.clone(),
            cfg.public_url.clone(),
        ))
    });

    let users_service = match (&tenants_repo, &users_repo, &new_secret_keys_repo) {
        (Some(t), Some(u), Some(sk)) => Some(Arc::new(
            crate::services::auth::users::service::UsersService::new(
                t.clone(),
                u.clone(),
                sk.clone(),
            ),
        )),
        _ => None,
    };

    let secret_keys_service = match (&new_secret_keys_repo, &users_repo) {
        (Some(sk), Some(u)) => Some(Arc::new(
            crate::services::auth::secret_keys::service::SecretKeysService::new(
                sk.clone(),
                u.clone(),
            ),
        )),
        _ => None,
    };

    let state = Arc::new(AppState {
        auth_repo,
        tenants_repo,
        users_repo,
        secret_keys_repo: new_secret_keys_repo,
        usage_repo,
        links_repo,
        domains_repo,
        apps_repo,
        config: cfg.clone(),
        facilitator,
        x402_price_tags,
        webhooks_repo,
        webhook_dispatcher,
        sdk_keys_repo,
        links_service,
        users_service,
        secret_keys_service,
    });

    // ── Build app: API + optional MCP on same port ──
    let mut app = axum::Router::new();

    #[cfg(feature = "api")]
    {
        app = api::router(state.clone())
            .with_state(state.clone())
            .merge(app);
    }

    #[cfg(feature = "mcp")]
    {
        if let (Some(links_svc), Some(auth)) = (&state.links_service, &state.auth_repo) {
            tracing::info!("MCP enabled at /mcp");
            app = app.merge(mcp::mcp_router(
                links_svc.clone(),
                auth.clone(),
                state.secret_keys_repo.clone(),
            ));
        }
    }

    let app = app
        .layer(RequestBodyLimitLayer::new(64 * 1024))
        .layer(CorsLayer::permissive());

    let addr = cfg.bind_addr();
    tracing::info!("Starting Rift on {addr}");

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind to address");

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await
    .expect("Server error");
}
