pub mod quota_response;
pub mod routes;
pub mod stripe_webhook;

use axum::middleware;
use axum::routing::{get, post};
use axum::Router;
use std::sync::Arc;

use super::auth::middleware::auth_gate;
use crate::app::AppState;

pub fn router(state: Arc<AppState>) -> Router<Arc<AppState>> {
    // Authenticated billing endpoints — live behind auth_gate so they inject
    // TenantId into handlers.
    let authenticated = Router::new()
        .route("/v1/billing/status", get(routes::get_billing_status))
        .route(
            "/v1/billing/stripe/checkout",
            post(routes::create_stripe_checkout),
        )
        .layer(middleware::from_fn_with_state(state, auth_gate));

    // Stripe webhook is public — auth comes from the HMAC signature over the
    // raw body, not a Bearer key. Must NOT go through auth_gate so the handler
    // receives the unmodified bytes for signature verification.
    let webhook = Router::new().route(
        "/v1/billing/webhooks/stripe",
        post(stripe_webhook::receive_stripe_webhook),
    );

    Router::new().merge(authenticated).merge(webhook)
}
