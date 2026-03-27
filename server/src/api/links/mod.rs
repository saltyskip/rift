pub mod models;
pub mod repo;
pub mod routes;

use axum::middleware;
use axum::routing::{get, post, put};
use axum::Router;
use std::sync::Arc;

use super::auth::middleware::{auth_gate, sdk_auth_gate};
use crate::api::AppState;
use crate::core::rate_limit::{rate_limit_middleware, RateLimiter};

pub fn router(state: Arc<AppState>) -> Router<Arc<AppState>> {
    // Authenticated routes (link management) — rl_live_ auth_gate injects TenantId.
    let authenticated = Router::new()
        .route("/v1/links", post(routes::create_link))
        .route("/v1/links", get(routes::list_links))
        .route("/v1/links/{link_id}/stats", get(routes::get_link_stats))
        .route(
            "/v1/links/{link_id}/timeseries",
            get(routes::get_link_timeseries),
        )
        .route(
            "/v1/links/{link_id}",
            put(routes::update_link).delete(routes::delete_link),
        )
        .route("/v1/attribution/link", put(routes::link_attribution))
        .layer(middleware::from_fn_with_state(state.clone(), auth_gate));

    // SDK-authenticated routes (attribution) — pk_live_ sdk_auth_gate
    let sdk = Router::new()
        .route("/v1/attribution/click", post(routes::attribution_click))
        .route("/v1/attribution/report", post(routes::attribution_report))
        .layer(middleware::from_fn_with_state(state, sdk_auth_gate));

    // Rate limiter for public endpoints: 120 req/min sustained, burst of 30.
    let limiter = Arc::new(RateLimiter::new(120, 30));

    // Public routes with rate limiting.
    let public = Router::new()
        .route("/__preview/theme/{theme_slug}", get(routes::preview_theme))
        .route(
            "/__preview/assets/{theme_slug}/{asset_name}",
            get(routes::preview_asset),
        )
        .route("/r/{link_id}", get(routes::resolve_link))
        .route("/{link_id}", get(routes::resolve_link_custom))
        .route("/llms.txt", get(routes::llms_txt))
        .layer(middleware::from_fn(rate_limit_middleware))
        .layer(axum::Extension(limiter));

    Router::new().merge(authenticated).merge(sdk).merge(public)
}
