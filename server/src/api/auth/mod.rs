pub mod keys;
pub mod middleware;
pub mod repo;
pub mod routes;

use axum::routing::{get, post};
use axum::Router;
use std::sync::Arc;

use crate::api::AppState;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/v1/auth/signup", post(routes::signup))
        .route("/v1/auth/verify", get(routes::verify_email))
}
