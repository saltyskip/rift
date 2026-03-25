pub mod models;
pub mod routes;

use axum::routing::get;
use axum::Router;
use std::sync::Arc;

use crate::api::AppState;

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/health", get(routes::health))
}
