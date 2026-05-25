pub mod models;
pub mod routes;

use axum::middleware;
use axum::routing::{get, post};
use axum::Router;
use std::sync::Arc;

use super::middleware::session_auth_gate;
use crate::app::AppState;

pub fn router(state: Arc<AppState>) -> Router<Arc<AppState>> {
    // Public — no auth, rate-limited inside the service layer.
    let public = Router::new()
        .route("/v1/auth/signin", post(routes::sign_in))
        .route(
            "/v1/auth/callback",
            get(routes::callback).post(routes::callback_confirm),
        );

    // Session-authed — `/me`, `/signout` are the dashboard's read/write
    // hooks. `/secret-keys/issue` is the instant-mint path used by the
    // "+ Create API key" button on `/account`.
    let session_protected = Router::new()
        .route("/v1/auth/me", get(routes::me))
        .route("/v1/auth/signout", post(routes::sign_out))
        .route("/v1/auth/secret-keys/issue", post(routes::issue_secret_key))
        .layer(middleware::from_fn_with_state(state, session_auth_gate));

    public.merge(session_protected)
}
