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
        .route(
            "/v1/billing/stripe/portal",
            post(routes::create_stripe_portal),
        )
        .route("/v1/billing/cancel", post(routes::cancel_subscription))
        .layer(middleware::from_fn_with_state(state, auth_gate));

    // Public magic-link endpoints. No bearer auth — identity is proven by
    // control of the emailed token. Rate limiting is inline in the handler.
    let public = Router::new()
        .route("/v1/billing/magic-link", post(routes::create_magic_link))
        .route("/v1/billing/go", get(routes::redeem_magic_link));

    // Stripe webhook is public — auth comes from the HMAC signature over the
    // raw body, not a Bearer key. Must NOT go through auth_gate so the handler
    // receives the unmodified bytes for signature verification.
    let webhook = Router::new().route(
        "/v1/billing/webhooks/stripe",
        post(stripe_webhook::receive_stripe_webhook),
    );

    Router::new()
        .merge(authenticated)
        .merge(public)
        .merge(webhook)
}
