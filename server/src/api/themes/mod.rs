pub mod models;
pub mod repo;
pub mod routes;

use axum::middleware;
use axum::routing::{delete, get, patch, post};
use axum::Router;
use std::sync::Arc;

use super::auth::middleware::auth_gate;
use crate::api::AppState;

pub fn router(state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .route("/v1/themes", post(routes::create_theme))
        .route("/v1/themes", get(routes::list_themes))
        .route("/v1/themes/{theme_id}", get(routes::get_theme))
        .route("/v1/themes/{theme_id}", patch(routes::update_theme))
        .route("/v1/themes/{theme_id}", delete(routes::delete_theme))
        .layer(middleware::from_fn_with_state(state, auth_gate))
}
