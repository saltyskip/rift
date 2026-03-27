pub mod models;
pub mod repo;
pub mod routes;

use axum::middleware;
use axum::routing::{delete, get, post, put};
use axum::Router;
use std::sync::Arc;

use super::auth::middleware::auth_gate;
use crate::api::AppState;

pub fn router(state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .route("/v1/domains", post(routes::create_domain))
        .route("/v1/domains", get(routes::list_domains))
        .route("/v1/domains/{domain}", delete(routes::delete_domain))
        .route("/v1/domains/{domain}/theme", put(routes::update_domain_theme))
        .route("/v1/domains/{domain}/verify", post(routes::verify_domain))
        .layer(middleware::from_fn_with_state(state, auth_gate))
}
