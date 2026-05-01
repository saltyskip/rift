pub mod models;
pub mod routes;

use axum::middleware;
use axum::routing::{delete, get, post};
use axum::Router;
use std::sync::Arc;

use super::middleware::auth_gate;
use crate::app::AppState;

pub fn router(state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .route("/v1/auth/users", post(routes::invite_user))
        .route("/v1/auth/users", get(routes::list_users))
        .route("/v1/auth/users/{user_id}", delete(routes::delete_user))
        .layer(middleware::from_fn_with_state(state, auth_gate))
}
