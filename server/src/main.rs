#[cfg(feature = "api")]
mod api;
mod app;
mod core;
mod error;
#[cfg(feature = "mcp")]
mod mcp;
mod migrations;
mod services;

/// Marker macro — see `lib.rs` for the canonical definition. Declared here
/// too so the bin target (which compiles `services/` independently of the
/// lib crate) can resolve `crate::impl_container!(...)` calls inside those
/// shared source files.
#[macro_export]
macro_rules! impl_container {
    ($name:ident) => {};
}

use std::sync::Arc;

use clap::{Parser, Subcommand};
use tower_http::cors::{AllowHeaders, AllowMethods, AllowOrigin, CorsLayer};
use tower_http::limit::RequestBodyLimitLayer;
use tracing_subscriber::prelude::*;
use tracing_subscriber::EnvFilter;

use x402_chain_eip155::{KnownNetworkEip155, V1Eip155Exact};
use x402_types::networks::USDC;

use crate::app::AppState;
use crate::core::config::Config;
use crate::services::affiliates::repo::AffiliatesRepo;
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
    // Mirrors the recommended config from
    // https://docs.sentry.io/platforms/rust/logs/ — ERROR fires both a
    // Sentry Event and a structured Log; INFO/WARN/DEBUG produce a
    // breadcrumb plus a Log; TRACE is dropped. Requires the `logs` feature
    // on the sentry crate and `enable_logs: true` on ClientOptions.
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::new("rift=info,tower_http=warn,hyper=warn,h2=warn,reqwest=warn,mongodb=warn")
    });
    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer())
        .with(
            sentry::integrations::tracing::layer().event_filter(|md| match *md.level() {
                tracing::Level::ERROR => {
                    sentry::integrations::tracing::EventFilter::Event
                        | sentry::integrations::tracing::EventFilter::Log
                }
                tracing::Level::TRACE => sentry::integrations::tracing::EventFilter::Ignore,
                _ => {
                    sentry::integrations::tracing::EventFilter::Breadcrumb
                        | sentry::integrations::tracing::EventFilter::Log
                }
            }),
        )
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
    // Initialise Sentry (no-op if SENTRY_DSN is empty). Release is the git
    // SHA injected via Docker build-arg; environment comes from ENVIRONMENT.
    let _sentry_guard = sentry::init(sentry::ClientOptions {
        dsn: cfg.sentry_dsn.parse().ok(),
        release: Some(cfg.git_sha.clone().into()),
        environment: Some(cfg.environment.clone().into()),
        traces_sample_rate: 0.2,
        enable_logs: true,
        send_default_pii: false,
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
        stripe_webhook_dedup_repo,
        tokens_repo,
        affiliates_repo,
        sessions_repo,
    ) = if cfg.mongo_uri.is_empty() {
        tracing::warn!(
            "MONGO_URI not set — auth, links, domains, apps, webhooks, sdk_keys, conversions, and affiliates disabled"
        );
        (
            None, None, None, None, None, None, None, None, None, None, None, None, None, None,
            None,
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
                let affiliates: Arc<dyn crate::services::affiliates::repo::AffiliatesRepository> =
                    Arc::new(AffiliatesRepo::new(&database).await);
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
                let stripe_dedup: Arc<
                    dyn crate::services::billing::repos::stripe_webhook_dedup::StripeWebhookDedupRepository,
                > = Arc::new(
                    crate::services::billing::repos::stripe_webhook_dedup::StripeWebhookDedupRepo::new(
                        &database,
                    )
                    .await,
                );
                let tokens: Arc<dyn crate::services::tokens::TokensRepository> =
                    Arc::new(crate::services::tokens::repo::TokensRepoMongo::new(&database).await);
                let sessions: Arc<dyn crate::services::auth::sessions::SessionsRepository> =
                    Arc::new(
                        crate::services::auth::sessions::repo::SessionsRepoMongo::new(&database)
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
                    Some(stripe_dedup),
                    Some(tokens),
                    Some(affiliates),
                    Some(sessions),
                )
            }
            None => {
                tracing::warn!(
                    "Failed to connect to MongoDB — auth, links, domains, apps, webhooks, sdk_keys, conversions, and affiliates disabled"
                );
                (
                    None, None, None, None, None, None, None, None, None, None, None, None, None,
                    None, None,
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

    // TokenService is a leaf (depends only on its repo) — construct early so
    // every domain service below can take an Arc into it.
    let tokens_service = tokens_repo
        .as_ref()
        .map(|r| Arc::new(crate::services::tokens::TokenService::new(r.clone())));

    // users_service is built after quota_service to enforce team-member
    // quota at the service layer — MCP and HTTP both benefit.

    let secret_keys_service = match (&secret_keys_repo, &users_repo, &tokens_service) {
        (Some(sk), Some(u), Some(tokens)) => Some(Arc::new(
            crate::services::auth::secret_keys::service::SecretKeysService::new(
                sk.clone(),
                u.clone(),
                tokens.clone(),
            ),
        )),
        _ => None,
    };

    let billing_service = tenants_repo.as_ref().map(|t| {
        Arc::new(crate::services::billing::service::BillingService::new(
            t.clone(),
        ))
    });

    let billing_handoff_service = match (&tokens_service, &tenants_repo) {
        (Some(tokens), Some(t)) => {
            let config = crate::services::billing::handoff::BillingHandoffConfig {
                resend_api_key: cfg.resend_api_key.clone(),
                resend_from_email: cfg.resend_from_email.clone(),
                public_url: cfg.public_url.clone(),
                marketing_url: cfg.marketing_url.clone(),
                stripe: crate::services::billing::stripe_client::StripeConfig {
                    secret_key: cfg.stripe_secret_key.clone(),
                    price_id_pro: cfg.stripe_price_id_pro.clone(),
                    price_id_business: cfg.stripe_price_id_business.clone(),
                    price_id_scale: cfg.stripe_price_id_scale.clone(),
                    success_url: cfg.stripe_success_url.clone(),
                    cancel_url: cfg.stripe_cancel_url.clone(),
                },
            };
            Some(Arc::new(
                crate::services::billing::handoff::BillingHandoffService::new(
                    tokens.clone(),
                    t.clone(),
                    config,
                ),
            ))
        }
        _ => None,
    };

    // conversions_service is built after quota_service — it takes both
    // billing (for retention bucketing) and quota (for TrackEvent check).

    let quota_service: Option<Arc<dyn crate::services::billing::quota::QuotaChecker>> = match (
        &billing_service,
        &event_counters_repo,
        &links_repo,
        &domains_repo,
        &users_repo,
        &webhooks_repo,
        &affiliates_repo,
    ) {
        (Some(b), Some(counters), Some(l), Some(d), Some(u), Some(w), Some(a)) => {
            let resource_counts = Arc::new(
                crate::services::billing::repos::resource_counts_adapter::RepoResourceCounts {
                    links: l.clone(),
                    domains: d.clone(),
                    users: u.clone(),
                    webhooks: w.clone(),
                    affiliates: a.clone(),
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
            let tiers = b.clone() as Arc<dyn crate::services::billing::service::TierResolver>;
            Some(Arc::new(
                crate::services::billing::quota::QuotaService::new(
                    tiers,
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
    let tier_resolver: Option<Arc<dyn crate::services::billing::service::TierResolver>> =
        billing_service
            .as_ref()
            .map(|b| b.clone() as Arc<dyn crate::services::billing::service::TierResolver>);

    let links_service = links_repo.as_ref().map(|repo| {
        Arc::new(crate::services::links::service::LinksService::new(
            crate::services::links::models::LinksServiceDeps {
                links_repo: repo.clone(),
                domains_repo: domains_repo.clone(),
                affiliates_repo: affiliates_repo.clone(),
                conversions_repo: conversions_repo.clone(),
                threat_feed: threat_feed.clone(),
                public_url: cfg.public_url.clone(),
                quota: quota_service.clone(),
                tiers: tier_resolver.clone(),
            },
        ))
    });

    let domains_service = domains_repo.as_ref().map(|r| {
        Arc::new(crate::services::domains::service::DomainsService::new(
            r.clone(),
            quota_service.clone(),
        ))
    });

    let webhooks_service = webhooks_repo.as_ref().map(|r| {
        Arc::new(crate::services::webhooks::service::WebhooksService::new(
            r.clone(),
            quota_service.clone(),
        ))
    });

    let affiliates_service = match (&affiliates_repo, &secret_keys_repo) {
        (Some(r), Some(sk)) => Some(Arc::new(
            crate::services::affiliates::service::AffiliatesService::new(
                r.clone(),
                sk.clone(),
                quota_service.clone(),
            ),
        )),
        _ => None,
    };

    let users_service = match (&tenants_service, &users_repo, &tokens_service) {
        (Some(t), Some(u), Some(tokens)) => Some(Arc::new(
            crate::services::auth::users::service::UsersService::new(
                t.clone(),
                u.clone(),
                tokens.clone(),
                quota_service.clone(),
            ),
        )),
        _ => None,
    };

    let sessions_service = match (&tokens_service, &sessions_repo, &users_repo, &users_service) {
        (Some(tokens), Some(sessions), Some(u), Some(users_svc)) => {
            let config = crate::services::auth::sessions::SessionsConfig {
                public_url: cfg.public_url.clone(),
                resend_api_key: cfg.resend_api_key.clone(),
                resend_from_email: cfg.resend_from_email.clone(),
            };
            Some(Arc::new(
                crate::services::auth::sessions::service::SessionsService::new(
                    tokens.clone(),
                    sessions.clone(),
                    u.clone(),
                    users_svc.clone(),
                    config,
                ),
            ))
        }
        _ => None,
    };

    let conversions_service = match (&conversions_repo, &links_repo) {
        (Some(c), Some(l)) => Some(Arc::new(ConversionsService::new(
            c.clone(),
            l.clone(),
            webhook_dispatcher.clone(),
            tier_resolver.clone(),
            quota_service.clone(),
        ))),
        _ => None,
    };

    let state = Arc::new(AppState {
        tenants_repo,
        stripe_webhook_dedup: stripe_webhook_dedup_repo,
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
        domains_service,
        webhooks_service,
        affiliates_service,
        users_service,
        secret_keys_service,
        sessions_service,
        conversions_service,
        billing_service,
        billing_handoff_service,
        tokens_service,
    });
    // quota_service is consumed by the per-domain services above; it's
    // intentionally not stored in AppState (no route-level callers).
    drop(quota_service);

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
        .layer(sentry::integrations::tower::SentryHttpLayer::new().enable_transaction())
        .layer(sentry::integrations::tower::NewSentryLayer::new_from_top())
        .layer(build_cors_layer(&cfg));

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

/// Build the global CORS layer.
///
/// Session cookies require `credentials: true` on the CORS response, which in
/// turn requires an explicit origin allowlist (CORS spec forbids `*` with
/// credentials). Two env vars control which origins are accepted:
///
/// - `ALLOWED_ORIGINS` — exact-match list (comma separated). E.g.
///   `https://riftl.ink,https://sandbox.riftl.ink`.
/// - `ALLOWED_ORIGIN_REGEX` — single regex that must fully match the Origin
///   header. Anchored automatically (`^...$`). Use when the origin set is
///   dynamic (Vercel preview URLs use `-` separators inside one DNS label,
///   so DNS-label boundary checks can't safely match them).
///
/// **Vercel previews carry a structural impersonation risk.** Vercel team
/// slugs are user-chosen and live in the same DNS label as the
/// project/branch (`{proj}-git-{branch}-{team}.vercel.app`). An attacker
/// can register a Vercel team named `evil-{your-team}` and any regex you
/// anchor on the team suffix will let their previews through too. Treat
/// `ALLOWED_ORIGIN_REGEX` as a convenience for non-production environments;
/// for prod, prefer either exact `ALLOWED_ORIGINS` entries or moving the
/// previews onto a custom subdomain (`*.preview.riftl.ink`) where normal
/// DNS-label rules apply.
///
/// Defaults: marketing URL + localhost dev ports if `ALLOWED_ORIGINS` is unset.
/// Existing API-key callers from arbitrary origins are unaffected because
/// bearer-token requests don't need `credentials: true`.
fn build_cors_layer(cfg: &Config) -> CorsLayer {
    let mut exact: Vec<axum::http::HeaderValue> = std::env::var("ALLOWED_ORIGINS")
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse().ok())
        .collect();

    if exact.is_empty() {
        for s in [
            cfg.marketing_url.as_str(),
            "http://localhost:3000",
            "http://localhost:3001",
        ] {
            if let Ok(v) = axum::http::HeaderValue::from_str(s) {
                if !exact.contains(&v) {
                    exact.push(v);
                }
            }
        }
    }

    let pattern = std::env::var("ALLOWED_ORIGIN_REGEX")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .and_then(|raw| {
            // Force full-match anchoring so callers can't accidentally write
            // a partial-match pattern that lets prefixes through.
            let anchored = format!("^(?:{})$", raw.trim());
            match regex::Regex::new(&anchored) {
                Ok(re) => Some(re),
                Err(e) => {
                    tracing::error!(error = %e, pattern = %raw, "invalid ALLOWED_ORIGIN_REGEX — ignored");
                    None
                }
            }
        });

    // `*` for methods/headers is forbidden when `credentials: true`; mirroring
    // the request value is the spec-friendly alternative.
    CorsLayer::new()
        .allow_origin(AllowOrigin::predicate(move |origin, _parts| {
            origin_matches(origin, &exact, pattern.as_ref())
        }))
        .allow_credentials(true)
        .allow_methods(AllowMethods::mirror_request())
        .allow_headers(AllowHeaders::mirror_request())
}

/// Decide whether `origin` is allowed. Exact-match wins; falls back to the
/// regex if configured. Pure function — see `main_tests.rs` for the cases.
fn origin_matches(
    origin: &axum::http::HeaderValue,
    exact: &[axum::http::HeaderValue],
    pattern: Option<&regex::Regex>,
) -> bool {
    if exact.iter().any(|e| e == origin) {
        return true;
    }
    let Some(re) = pattern else {
        return false;
    };
    let Ok(origin_str) = origin.to_str() else {
        return false;
    };
    re.is_match(origin_str)
}

#[cfg(test)]
#[path = "main_tests.rs"]
mod tests;
