pub mod routes;

use axum::middleware;
use axum::routing::{delete, get, post};
use axum::Router;
use std::sync::Arc;

use super::auth::middleware::auth_gate;
use crate::app::AppState;

pub fn router(state: Arc<AppState>) -> Router<Arc<AppState>> {
    // Authenticated CRUD endpoints (require rl_live_ API key).
    let authenticated = Router::new()
        .route("/v1/sources", post(routes::create_source))
        .route("/v1/sources", get(routes::list_sources))
        .route("/v1/sources/{id}", get(routes::get_source))
        .route("/v1/sources/{id}", delete(routes::delete_source))
        .layer(middleware::from_fn_with_state(state, auth_gate));

    // Public webhook receiver — auth is the opaque url_token in the URL.
    let public = Router::new().route("/w/{token}", post(routes::receive_webhook));

    authenticated.merge(public)
}
