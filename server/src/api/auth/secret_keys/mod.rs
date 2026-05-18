pub mod models;
pub mod routes;

use axum::middleware;
use axum::routing::{delete, get, post};
use axum::Router;
use std::sync::Arc;

use super::middleware::{auth_gate, combined_auth_gate};
use crate::app::AppState;

pub fn router(state: Arc<AppState>) -> Router<Arc<AppState>> {
    // Public — team-invite acceptance from email.
    let public = Router::new().route("/v1/auth/verify", get(routes::verify_email));

    // API-key-only: the email-code dance ("mint a key for a teammate")
    // requires confirming via a code sent to the teammate's email — only
    // meaningful when someone with a key wants to provision another. The
    // session-authed `/v1/auth/secret-keys/issue` (handled in `sessions/`)
    // covers the dashboard's "+ Create API key" button without a round-trip.
    let key_only = Router::new()
        .route("/v1/auth/secret-keys", post(routes::request_create_key))
        .route(
            "/v1/auth/secret-keys/confirm",
            post(routes::confirm_create_key),
        )
        .layer(middleware::from_fn_with_state(state.clone(), auth_gate));

    // Either auth method works — these are listing/revocation primitives
    // useful from both the dashboard (session cookie) and from automation
    // (API key). Affiliate-scope checks still run on the API-key path
    // inside `combined_auth_gate`.
    let combined = Router::new()
        .route("/v1/auth/secret-keys", get(routes::list_secret_keys))
        .route(
            "/v1/auth/secret-keys/{key_id}",
            delete(routes::delete_secret_key),
        )
        .layer(middleware::from_fn_with_state(state, combined_auth_gate));

    public.merge(key_only).merge(combined)
}
