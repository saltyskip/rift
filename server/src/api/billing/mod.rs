pub mod quota_response;
pub mod routes;

use axum::middleware;
use axum::routing::get;
use axum::Router;
use std::sync::Arc;

use super::auth::middleware::auth_gate;
use crate::app::AppState;

pub fn router(state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .route("/v1/billing/status", get(routes::get_billing_status))
        .layer(middleware::from_fn_with_state(state, auth_gate))
}
