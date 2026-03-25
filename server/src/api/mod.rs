pub mod apps;
pub mod auth;
pub mod domains;
pub mod health;
pub mod links;
pub mod sdk_keys;
pub mod webhooks;

use axum::routing::get;
use axum::Router;
use std::sync::Arc;
use utoipa::OpenApi;

use std::sync::Arc as StdArc;

use crate::api::apps::repo::AppsRepository;
use crate::api::auth::repo::AuthRepository;
use crate::api::domains::repo::DomainsRepository;
use crate::api::links::repo::LinksRepository;
use crate::api::sdk_keys::repo::SdkKeysRepository;
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
        description = "Deep links for humans and agents",
        contact(name = "Rift"),
    ),
    paths(
        health::routes::health,
        auth::routes::signup,
        auth::routes::verify_email,
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
        sdk_keys::routes::create_sdk_key,
        sdk_keys::routes::list_sdk_keys,
        sdk_keys::routes::revoke_sdk_key,
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
        auth::routes::SignupRequest,
        auth::routes::SignupResponse,
        links::models::CreateLinkRequest,
        links::models::CreateLinkResponse,
        links::models::UpdateLinkRequest,
        links::models::LinkDetail,
        links::models::ListLinksResponse,
        links::models::LinkStatsResponse,
        links::models::ResolvedLink,
        links::models::ClickRequest,
        links::models::AttributionReportRequest,
        links::models::LinkAttributionRequest,
        links::models::AttributionResponse,
        links::models::AgentContext,
        links::models::TimeseriesDataPoint,
        links::models::TimeseriesResponse,
        sdk_keys::models::CreateSdkKeyRequest,
        sdk_keys::models::CreateSdkKeyResponse,
        sdk_keys::models::SdkKeyDetail,
        sdk_keys::models::ListSdkKeysResponse,
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
        (name = "Authentication", description = "Email-based API key signup and verification"),
        (name = "System", description = "Health checks and operational endpoints"),
        (name = "Domains", description = "Custom domain management"),
        (name = "Apps", description = "App configuration and association files"),
        (name = "Webhooks", description = "Webhook management for real-time event notifications"),
        (name = "SDK Keys", description = "Publishable SDK key management"),
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
        .merge(auth::router())
        .merge(links::router(state.clone()))
        .merge(sdk_keys::router(state.clone()))
        .merge(domains::router(state.clone()))
        .merge(apps::router(state.clone()))
        .merge(webhooks::router(state.clone()))
        .merge(openapi_json)
        .merge(sdk)
}
