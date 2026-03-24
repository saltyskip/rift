pub mod dispatcher;
pub mod models;
pub mod repo;
pub mod routes;

use axum::middleware;
use axum::routing::{delete, get, post};
use axum::Router;
use std::sync::Arc;

use super::auth::middleware::auth_gate;
use crate::api::AppState;

pub fn router(state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .route("/v1/webhooks", post(routes::create_webhook))
        .route("/v1/webhooks", get(routes::list_webhooks))
        .route("/v1/webhooks/{webhook_id}", delete(routes::delete_webhook))
        .layer(middleware::from_fn_with_state(state, auth_gate))
}
