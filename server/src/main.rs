#[cfg(feature = "api")]
mod api;
mod app;
mod core;
mod error;
#[cfg(feature = "mcp")]
mod mcp;
mod migrations;
mod services;

use std::sync::Arc;

use clap::{Parser, Subcommand};
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
use crate::services::auth::secret_keys::repo::SecretKeysRepo;
use crate::services::auth::tenants::repo::TenantsRepo;
use crate::services::auth::usage::repo::UsageRepo;
use crate::services::auth::users::repo::UsersRepo;
use crate::services::conversions::repo::ConversionsRepo;
use crate::services::conversions::service::ConversionsService;
use crate::services::domains::repo::DomainsRepo;
use crate::services::links::repo::LinksRepo;
use crate::services::webhooks::dispatcher::RiftWebhookDispatcher;
use crate::services::webhooks::repo::WebhooksRepo;

#[derive(Parser)]
#[command(name = "rift", about = "Deep links for humans and agents")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Run a named migration (dry run by default)
    Migrate {
        /// Migration name (e.g. m001_auth_split)
        #[arg(long)]
        name: Option<String>,

        /// List available migrations
        #[arg(long)]
        list: bool,

        /// Actually apply the migration (default is dry run)
        #[arg(long)]
        apply: bool,
    },
}

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv();
    let cfg = Config::from_env();

    // Initialise tracing with Sentry layer (respects RUST_LOG env var).
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer())
        .with(sentry_tracing::layer())
        .init();

    let cli = Cli::parse();

    match cli.command {
        Some(Command::Migrate { list: true, .. }) => {
            println!("Available migrations:");
            for m in migrations::all() {
                println!("  {} — {}", m.id(), m.description());
            }
        }
        Some(Command::Migrate {
            name: Some(name),
            apply,
            ..
        }) => {
            let migration = match migrations::get_by_name(&name) {
                Some(m) => m,
                None => {
                    eprintln!("Unknown migration: {name}");
                    eprintln!("Run with --list to see available migrations.");
                    std::process::exit(1);
                }
            };

            let db = match core::db::connect(&cfg.mongo_uri, &cfg.mongo_db).await {
                Some(db) => db,
                None => {
                    eprintln!("Failed to connect to MongoDB");
                    std::process::exit(1);
                }
            };

            if apply {
                println!(
                    "Applying migration: {} — {}",
                    migration.id(),
                    migration.description()
                );
                match migration.run(&db, false).await {
                    Ok(()) => println!("Migration {} completed successfully", migration.id()),
                    Err(e) => {
                        eprintln!("Migration failed: {e}");
                        std::process::exit(1);
                    }
                }
            } else {
                println!(
                    "Dry run for migration: {} — {}",
                    migration.id(),
                    migration.description()
                );
                println!("(pass --apply to execute)\n");
                match migration.run(&db, true).await {
                    Ok(()) => println!("\nDry run complete. No changes were made."),
                    Err(e) => {
                        eprintln!("Dry run failed: {e}");
                        std::process::exit(1);
                    }
                }
            }
        }
        Some(Command::Migrate { .. }) => {
            eprintln!("Provide --name <migration> or --list");
            std::process::exit(1);
        }

        // Default: run the server
        None => run_server(cfg).await,
    }
}

async fn run_server(cfg: Config) {
    // Initialise Sentry (no-op if SENTRY_DSN is empty).
    let _sentry_guard = sentry::init(sentry::ClientOptions {
        dsn: cfg.sentry_dsn.parse().ok(),
        traces_sample_rate: 0.2,
        ..sentry::ClientOptions::default()
    });

    // Connect to MongoDB (optional — server boots without it).
    let (
        tenants_repo,
        users_repo,
        secret_keys_repo,
        usage_repo,
        links_repo,
        domains_repo,
        apps_repo,
        webhooks_repo,
        sdk_keys_repo,
        conversions_repo,
        event_counters_repo,
    ) = if cfg.mongo_uri.is_empty() {
        tracing::warn!(
            "MONGO_URI not set — auth, links, domains, apps, webhooks, sdk_keys, and conversions disabled"
        );
        (
            None, None, None, None, None, None, None, None, None, None, None,
        )
    } else {
        match core::db::connect(&cfg.mongo_uri, &cfg.mongo_db).await {
            Some(database) => {
                tracing::info!(uri = %cfg.mongo_uri, db = %cfg.mongo_db, "Connected to MongoDB");
                let tenants: Arc<dyn crate::services::auth::tenants::repo::TenantsRepository> =
                    Arc::new(TenantsRepo::new(&database).await);
                let users: Arc<dyn crate::services::auth::users::repo::UsersRepository> =
                    Arc::new(UsersRepo::new(&database).await);
                let secret_keys: Arc<
                    dyn crate::services::auth::secret_keys::repo::SecretKeysRepository,
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
                let conversions: Arc<
                    dyn crate::services::conversions::repo::ConversionsRepository,
                > = Arc::new(ConversionsRepo::new(&database).await);
                let event_counters: Arc<
                    dyn crate::services::billing::repos::event_counters::EventCountersRepository,
                > = Arc::new(
                    crate::services::billing::repos::event_counters::EventCountersRepo::new(
                        &database,
                    )
                    .await,
                );
                (
                    Some(tenants),
                    Some(users),
                    Some(secret_keys),
                    Some(usage),
                    Some(links),
                    Some(domains),
                    Some(apps),
                    Some(webhooks),
                    Some(sdk_keys),
                    Some(conversions),
                    Some(event_counters),
                )
            }
            None => {
                tracing::warn!(
                    "Failed to connect to MongoDB — auth, links, domains, apps, webhooks, sdk_keys, and conversions disabled"
                );
                (
                    None, None, None, None, None, None, None, None, None, None, None,
                )
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

    // links_service is constructed after quota_service (below) — service-
    // layer quota enforcement is required for MCP parity.

    let tenants_service = tenants_repo
        .as_ref()
        .map(|t| Arc::new(crate::services::auth::tenants::service::TenantsService::new(t.clone())));

    // users_service is built after quota_service to enforce team-member
    // quota at the service layer — MCP and HTTP both benefit.

    let secret_keys_service = match (&secret_keys_repo, &users_repo) {
        (Some(sk), Some(u)) => Some(Arc::new(
            crate::services::auth::secret_keys::service::SecretKeysService::new(
                sk.clone(),
                u.clone(),
            ),
        )),
        _ => None,
    };

    let billing_service = tenants_repo.as_ref().map(|t| {
        Arc::new(crate::services::billing::service::BillingService::new(
            t.clone(),
        ))
    });

    // conversions_service is built after quota_service — it takes both
    // billing (for retention bucketing) and quota (for TrackEvent check).

    let quota_service = match (
        &billing_service,
        &event_counters_repo,
        &links_repo,
        &domains_repo,
        &users_repo,
        &webhooks_repo,
    ) {
        (Some(b), Some(counters), Some(l), Some(d), Some(u), Some(w)) => {
            let resource_counts = Arc::new(
                crate::services::billing::repos::resource_counts_adapter::RepoResourceCounts {
                    links: l.clone(),
                    domains: d.clone(),
                    users: u.clone(),
                    webhooks: w.clone(),
                },
            )
                as Arc<dyn crate::services::billing::quota::ResourceCounts>;
            // QUOTA_ENFORCEMENT=enforce flips hard rejection on. Default is
            // log-only so accidental rollouts don't start 402ing real
            // customers before the counters are verified in prod.
            let mode = crate::services::billing::quota::EnforcementMode::from_env_str(
                &std::env::var("QUOTA_ENFORCEMENT").unwrap_or_default(),
            );
            tracing::info!(mode = ?mode, "quota_enforcement_mode");
            Some(Arc::new(
                crate::services::billing::quota::QuotaService::new(
                    b.clone(),
                    counters.clone(),
                    resource_counts,
                    mode,
                ),
            ))
        }
        _ => None,
    };

    // LinksService + UsersService need quota_service, so construct them
    // after the match above. Service-layer enforcement is the contract:
    // MCP tool invocations hit the same checks as HTTP route handlers.
    let links_service = links_repo.as_ref().map(|repo| {
        Arc::new(crate::services::links::service::LinksService::new(
            repo.clone(),
            domains_repo.clone(),
            threat_feed.clone(),
            cfg.public_url.clone(),
            quota_service.clone(),
        ))
    });

    let users_service = match (&tenants_service, &users_repo, &secret_keys_repo) {
        (Some(t), Some(u), Some(sk)) => Some(Arc::new(
            crate::services::auth::users::service::UsersService::new(
                t.clone(),
                u.clone(),
                sk.clone(),
                quota_service.clone(),
            ),
        )),
        _ => None,
    };

    let conversions_service = match (&conversions_repo, &links_repo) {
        (Some(c), Some(l)) => Some(Arc::new(ConversionsService::new(
            c.clone(),
            l.clone(),
            webhook_dispatcher.clone(),
            billing_service.clone(),
            quota_service.clone(),
        ))),
        _ => None,
    };

    let state = Arc::new(AppState {
        secret_keys_repo,
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
        conversions_repo,
        links_service,
        users_service,
        secret_keys_service,
        conversions_service,
        billing_service,
        quota_service,
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
        if let (Some(links_svc), Some(sk_repo)) = (&state.links_service, &state.secret_keys_repo) {
            tracing::info!("MCP enabled at /mcp");
            app = app.merge(mcp::mcp_router(
                links_svc.clone(),
                sk_repo.clone(),
                state.conversions_repo.clone(),
                state.config.public_url.clone(),
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
