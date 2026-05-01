pub mod models;
pub mod routes;

use axum::middleware;
use axum::routing::{delete, get, post};
use axum::Router;
use std::sync::Arc;

use super::auth::middleware::{auth_gate, sdk_auth_gate};
use crate::app::AppState;

pub fn router(state: Arc<AppState>) -> Router<Arc<AppState>> {
    // Authenticated CRUD endpoints (require rl_live_ API key).
    let authenticated = Router::new()
        .route("/v1/sources", post(routes::create_source))
        .route("/v1/sources", get(routes::list_sources))
        .route("/v1/sources/{id}", get(routes::get_source))
        .route("/v1/sources/{id}", delete(routes::delete_source))
        .layer(middleware::from_fn_with_state(state.clone(), auth_gate));

    // SDK-authenticated conversion tracking (pk_live_ bearer).
    // Mobile SDKs use this instead of the webhook URL so they don't need
    // a separate source URL in the binary.
    let sdk = Router::new()
        .route(
            "/v1/attribution/convert",
            post(routes::sdk_track_conversion),
        )
        .layer(middleware::from_fn_with_state(state, sdk_auth_gate));

    // Public webhook receiver — auth is the opaque url_token in the URL.
    let public = Router::new().route("/w/{token}", post(routes::receive_webhook));

    authenticated.merge(sdk).merge(public)
}
