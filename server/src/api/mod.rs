pub mod apps;
pub mod auth;
pub mod domains;
pub mod health;
pub mod links;
pub mod webhooks;

use axum::routing::get;
use axum::Router;
use std::sync::Arc;
use utoipa::OpenApi;

use std::sync::Arc as StdArc;

use crate::api::apps::repo::AppsRepository;
use crate::api::auth::publishable_keys::repo::SdkKeysRepository;
use crate::api::auth::secret_keys::repo::AuthRepository;
use crate::api::domains::repo::DomainsRepository;
use crate::api::links::repo::LinksRepository;
use crate::api::webhooks::repo::WebhooksRepository;
use crate::core::config::Config;
use crate::core::threat_feed::ThreatFeed;
use crate::core::webhook_dispatcher::WebhookDispatcher;

use x402_types::proto::v1;

use crate::core::cdp::CdpFacilitator;

/// Shared application state available to all route handlers.
pub struct AppState {
    pub auth_repo: Option<StdArc<dyn AuthRepository>>,
    pub links_repo: Option<StdArc<dyn LinksRepository>>,
    pub domains_repo: Option<StdArc<dyn DomainsRepository>>,
    pub apps_repo: Option<StdArc<dyn AppsRepository>>,
    pub config: Config,
    pub facilitator: Option<CdpFacilitator>,
    pub x402_price_tags: Vec<v1::PriceTag>,
    pub threat_feed: ThreatFeed,
    pub webhooks_repo: Option<StdArc<dyn WebhooksRepository>>,
    pub webhook_dispatcher: Option<StdArc<dyn WebhookDispatcher>>,
    pub sdk_keys_repo: Option<StdArc<dyn SdkKeysRepository>>,
}

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Rift API",
        version = "0.1.0",
        description = "Deep links for humans and agents.\n\nRift creates smart, cross-platform deep links with structured context for AI agents. Each link carries per-platform destinations, metadata, and an optional `agent_context` that makes links machine-readable.\n\n## Authentication\n\nSign up at `POST /v1/auth/signup` to get an API key (starts with `rl_live_`). Include it as a Bearer token in the Authorization header. Client SDKs use publishable keys (`pk_live_`) for click tracking and attribution.\n\n## Quick start\n\n1. `POST /v1/auth/signup` — get an API key\n2. `POST /v1/links` — create your first link\n3. `GET /r/{link_id}` — resolve it (redirect for browsers, JSON for agents)\n\n## Content negotiation\n\nThe resolve endpoint (`GET /r/{link_id}`) is content-negotiated. Browsers receive a redirect or landing page. Requests with `Accept: application/json` receive structured link data including `agent_context` and a `_rift_meta` trust envelope.",
        contact(name = "Rift", url = "https://riftl.ink"),
    ),
    paths(
        health::routes::health,
        auth::secret_keys::routes::signup,
        auth::secret_keys::routes::verify_email,
        links::routes::create_link,
        links::routes::list_links,
        links::routes::get_link_stats,
        links::routes::resolve_link,
        links::routes::resolve_link_custom,
        links::routes::attribution_click,
        links::routes::attribution_report,
        links::routes::link_attribution,
        links::routes::get_link_timeseries,
        links::routes::update_link,
        links::routes::delete_link,
        auth::publishable_keys::routes::create_sdk_key,
        auth::publishable_keys::routes::list_sdk_keys,
        auth::publishable_keys::routes::revoke_sdk_key,
        webhooks::routes::create_webhook,
        webhooks::routes::list_webhooks,
        webhooks::routes::delete_webhook,
        webhooks::routes::patch_webhook,
        domains::routes::create_domain,
        domains::routes::list_domains,
        domains::routes::delete_domain,
        domains::routes::verify_domain,
        apps::routes::create_app,
        apps::routes::list_apps,
        apps::routes::delete_app,
        apps::routes::serve_aasa,
        apps::routes::serve_assetlinks,
    ),
    components(schemas(
        health::models::HealthResponse,
        crate::error::ErrorResponse,
        auth::secret_keys::routes::SignupRequest,
        auth::secret_keys::routes::SignupResponse,
        links::models::CreateLinkRequest,
        links::models::CreateLinkResponse,
        links::models::UpdateLinkRequest,
        links::models::LinkDetail,
        links::models::ListLinksResponse,
        links::models::LinkStatsResponse,
        links::models::ResolvedLink,
        links::models::RiftMeta,
        links::models::ClickRequest,
        links::models::AttributionReportRequest,
        links::models::LinkAttributionRequest,
        links::models::AttributionResponse,
        links::models::AgentContext,
        links::models::TimeseriesDataPoint,
        links::models::TimeseriesResponse,
        auth::publishable_keys::models::CreateSdkKeyRequest,
        auth::publishable_keys::models::CreateSdkKeyResponse,
        auth::publishable_keys::models::SdkKeyDetail,
        auth::publishable_keys::models::ListSdkKeysResponse,
        domains::models::CreateDomainRequest,
        domains::models::CreateDomainResponse,
        domains::models::DomainDetail,
        domains::models::VerifyDomainResponse,
        apps::models::CreateAppRequest,
        apps::models::AppDetail,
        webhooks::models::CreateWebhookRequest,
        webhooks::models::CreateWebhookResponse,
        webhooks::models::WebhookDetail,
        webhooks::models::ListWebhooksResponse,
        webhooks::models::UpdateWebhookRequest,
        webhooks::models::WebhookEventType,
    )),
    security(
        ("api_key" = []),
        ("x402" = []),
    ),
    tags(
        (name = "Links", description = "Create, list, and resolve deep links"),
        (name = "Attribution", description = "Track installs and attribute them to links"),
        (name = "Authentication", description = "API key signup, verification, and publishable key management"),
        (name = "System", description = "Health checks and operational endpoints"),
        (name = "Domains", description = "Custom domain management"),
        (name = "Apps", description = "App configuration and association files"),
        (name = "Webhooks", description = "Webhook management for real-time event notifications"),
    )
)]
struct ApiDoc;

async fn serve_rift_js() -> impl axum::response::IntoResponse {
    (
        axum::http::StatusCode::OK,
        [
            (
                axum::http::header::CONTENT_TYPE,
                "application/javascript; charset=utf-8",
            ),
            (axum::http::header::CACHE_CONTROL, "public, max-age=3600"),
        ],
        include_str!("../sdk/rift.js"),
    )
}

pub fn router(state: Arc<AppState>) -> Router<Arc<AppState>> {
    let spec = ApiDoc::openapi();
    let openapi_json = Router::new().route(
        "/openapi.json",
        get(move || async move { axum::Json(spec) }),
    );

    let sdk = Router::new().route("/sdk/rift.js", get(serve_rift_js));

    health::router()
        .merge(auth::router(state.clone()))
        .merge(links::router(state.clone()))
        .merge(domains::router(state.clone()))
        .merge(apps::router(state.clone()))
        .merge(webhooks::router(state.clone()))
        .merge(openapi_json)
        .merge(sdk)
}
