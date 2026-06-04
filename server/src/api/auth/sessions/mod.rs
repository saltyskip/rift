pub mod models;
pub mod routes;

use axum::middleware;
use axum::routing::{get, post};
use axum::Router;
use std::sync::Arc;

use super::middleware::{require_auth, SESSION};
use crate::app::AppState;

pub fn router(state: Arc<AppState>) -> Router<Arc<AppState>> {
    // Public — no auth, rate-limited inside the service layer. `cli/start`
    // just validates the loopback redirect and bounces the browser to the
    // dashboard's `/cli/authorize` page (which owns the sign-in UI).
    let public = Router::new()
        .route("/v1/auth/signin", post(routes::sign_in))
        .route(
            "/v1/auth/callback",
            get(routes::callback).post(routes::callback_confirm),
        )
        .route("/v1/auth/cli/start", get(routes::cli_start));

    // Session-authed — `/me`, `/signout` are the dashboard's read/write
    // hooks. `/secret-keys/issue` is the instant-mint path used by the
    // "+ Create API key" button on `/account`. `/cli/authorize` mints a
    // `rift-cli` session for the CLI's browser login flow.
    let session_protected = Router::new()
        .route("/v1/auth/me", get(routes::me))
        .route("/v1/auth/signout", post(routes::sign_out))
        .route("/v1/auth/secret-keys/issue", post(routes::issue_secret_key))
        .route("/v1/auth/cli/authorize", post(routes::cli_authorize))
        .layer(middleware::from_fn_with_state(state, require_auth(SESSION)));

    public.merge(session_protected)
}
