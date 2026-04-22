pub mod routes;

use axum::middleware;
use axum::routing::{get, post};
use axum::Router;
use std::sync::Arc;

use super::auth::middleware::auth_gate;
use crate::app::AppState;

pub fn router(state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .route("/v1/billing/status", get(routes::get_billing_status))
        .route(
            "/v1/billing/stripe/checkout",
            post(routes::create_stripe_checkout),
        )
        .layer(middleware::from_fn_with_state(state, auth_gate))
}
