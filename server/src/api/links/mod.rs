pub mod models;
pub mod repo;
pub mod routes;

use axum::middleware;
use axum::routing::{get, post, put};
use axum::Router;
use std::sync::Arc;

use super::auth::middleware::auth_gate;
use crate::api::AppState;
use crate::core::rate_limit::{rate_limit_middleware, RateLimiter};

pub fn router(state: Arc<AppState>) -> Router<Arc<AppState>> {
    // Authenticated routes (link management) — auth_gate injects TenantId.
    let authenticated = Router::new()
        .route("/v1/links", post(routes::create_link))
        .route("/v1/links", get(routes::list_links))
        .route("/v1/links/{link_id}/stats", get(routes::get_link_stats))
        .route(
            "/v1/links/{link_id}/timeseries",
            get(routes::get_link_timeseries),
        )
        .route("/v1/attribution/link", put(routes::link_attribution))
        .layer(middleware::from_fn_with_state(state, auth_gate));

    // Rate limiter for public endpoints: 120 req/min sustained, burst of 30.
    let limiter = Arc::new(RateLimiter::new(120, 30));

    // Public routes with rate limiting.
    // Layer order: middleware runs first (outer), extension provides the limiter (inner).
    let public = Router::new()
        .route("/r/{link_id}", get(routes::resolve_link))
        .route("/{link_id}", get(routes::resolve_link_custom))
        .route("/v1/attribution", post(routes::report_attribution))
        .route("/v1/deferred", post(routes::resolve_deferred))
        .route("/v1/sdk/click", post(routes::sdk_click))
        .layer(middleware::from_fn(rate_limit_middleware))
        .layer(axum::Extension(limiter));

    Router::new().merge(authenticated).merge(public)
}
