pub mod models;
pub mod repo;
pub mod routes;

use axum::middleware;
use axum::routing::{delete, get, post};
use axum::Router;
use std::sync::Arc;

use super::middleware::auth_gate;
use crate::api::AppState;

pub fn router(state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .route("/v1/auth/publishable-keys", post(routes::create_sdk_key))
        .route("/v1/auth/publishable-keys", get(routes::list_sdk_keys))
        .route(
            "/v1/auth/publishable-keys/{key_id}",
            delete(routes::revoke_sdk_key),
        )
        .layer(middleware::from_fn_with_state(state, auth_gate))
}
