pub mod models;
pub mod routes;

use axum::middleware;
use axum::routing::{delete, get, post};
use axum::Router;
use std::sync::Arc;

use super::middleware::auth_gate;
use crate::app::AppState;

pub fn router(state: Arc<AppState>) -> Router<Arc<AppState>> {
    // Public routes (no auth)
    let public = Router::new()
        .route("/v1/auth/signup", post(routes::signup))
        .route("/v1/auth/verify", get(routes::verify_email));

    // Authenticated routes (behind auth_gate)
    let authenticated = Router::new()
        .route("/v1/auth/secret-keys", post(routes::request_create_key))
        .route(
            "/v1/auth/secret-keys/confirm",
            post(routes::confirm_create_key),
        )
        .route("/v1/auth/secret-keys", get(routes::list_secret_keys))
        .route(
            "/v1/auth/secret-keys/{key_id}",
            delete(routes::delete_secret_key),
        )
        .layer(middleware::from_fn_with_state(state, auth_gate));

    public.merge(authenticated)
}
