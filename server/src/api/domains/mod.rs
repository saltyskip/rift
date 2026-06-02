pub mod routes;

use axum::middleware;
use axum::routing::{delete, get, post};
use axum::Router;
use std::sync::Arc;

use super::auth::middleware::{auth_gate, auth_gate_dual_read};
use crate::app::AppState;

pub fn router(state: Arc<AppState>) -> Router<Arc<AppState>> {
    // Mutating routes are secret-key only (`auth_gate`).
    let mutating = Router::new()
        .route("/v1/domains", post(routes::create_domain))
        .route("/v1/domains/{domain}", delete(routes::delete_domain))
        .route("/v1/domains/{domain}/verify", post(routes::verify_domain))
        .layer(middleware::from_fn_with_state(state.clone(), auth_gate));

    // Listing is readable by both the dashboard/CLI (secret key) and the
    // mobile SDK (publishable key) — the SDK fetches the tenant's verified
    // domains to validate deferred-deep-link clipboard hosts.
    let read = Router::new()
        .route("/v1/domains", get(routes::list_domains))
        .layer(middleware::from_fn_with_state(state, auth_gate_dual_read));

    mutating.merge(read)
}
