pub mod routes;

use axum::middleware;
use axum::routing::{delete, get, post};
use axum::Router;
use std::sync::Arc;

use super::auth::middleware::{require_auth, ANONYMOUS, PUBLISHABLE, SECRET, SESSION, X402};
use crate::app::AppState;

pub fn router(state: Arc<AppState>) -> Router<Arc<AppState>> {
    // Mutating routes take the standard authenticated policy (session / secret
    // key / x402).
    let mutating = Router::new()
        .route("/v1/domains", post(routes::create_domain))
        .route("/v1/domains/{domain}", delete(routes::delete_domain))
        .route("/v1/domains/{domain}/verify", post(routes::verify_domain))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            require_auth(SESSION | SECRET | X402 | ANONYMOUS),
        ));

    // Listing is readable by the dashboard/CLI (session or secret key) and the
    // mobile SDK (publishable key) — the SDK fetches the tenant's verified
    // domains to validate deferred-deep-link clipboard hosts.
    let read = Router::new()
        .route("/v1/domains", get(routes::list_domains))
        .layer(middleware::from_fn_with_state(
            state,
            require_auth(SESSION | SECRET | PUBLISHABLE),
        ));

    mutating.merge(read)
}
