pub mod routes;

use axum::middleware;
use axum::routing::get;
use axum::Router;
use std::sync::Arc;

use super::middleware::{require_auth, SECRET, SESSION};
use crate::app::AppState;

pub fn router(state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/v1/tenant/branding",
            get(routes::get_branding).put(routes::update_branding),
        )
        .layer(middleware::from_fn_with_state(
            state,
            // Branding is account configuration — require a full credential
            // (session or secret key), not anonymous/x402.
            require_auth(SESSION | SECRET),
        ))
}
