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

use crate::app::AppState;

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
        crate::services::links::models::CreateLinkRequest,
        crate::services::links::models::CreateLinkResponse,
        crate::services::links::models::UpdateLinkRequest,
        crate::services::links::models::LinkDetail,
        crate::services::links::models::ListLinksResponse,
        crate::services::links::models::LinkStatsResponse,
        crate::services::links::models::ResolvedLink,
        crate::services::links::models::RiftMeta,
        crate::services::links::models::ClickRequest,
        crate::services::links::models::AttributionReportRequest,
        crate::services::links::models::LinkAttributionRequest,
        crate::services::links::models::AttributionResponse,
        crate::services::links::models::AgentContext,
        crate::services::links::models::TimeseriesDataPoint,
        crate::services::links::models::TimeseriesResponse,
        crate::services::auth::publishable_keys::models::CreateSdkKeyRequest,
        crate::services::auth::publishable_keys::models::CreateSdkKeyResponse,
        crate::services::auth::publishable_keys::models::SdkKeyDetail,
        crate::services::auth::publishable_keys::models::ListSdkKeysResponse,
        crate::services::domains::models::CreateDomainRequest,
        crate::services::domains::models::CreateDomainResponse,
        crate::services::domains::models::DomainDetail,
        crate::services::domains::models::VerifyDomainResponse,
        crate::services::apps::models::CreateAppRequest,
        crate::services::apps::models::AppDetail,
        crate::services::webhooks::models::CreateWebhookRequest,
        crate::services::webhooks::models::CreateWebhookResponse,
        crate::services::webhooks::models::WebhookDetail,
        crate::services::webhooks::models::ListWebhooksResponse,
        crate::services::webhooks::models::UpdateWebhookRequest,
        crate::services::webhooks::models::WebhookEventType,
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
