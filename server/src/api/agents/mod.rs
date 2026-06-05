pub mod routes;

use axum::middleware;
use axum::routing::post;
use axum::Router;
use std::sync::Arc;

use super::auth::middleware::{require_auth, SECRET, X402};
use crate::app::AppState;

pub fn router(state: Arc<AppState>) -> Router<Arc<AppState>> {
    // Server-to-server ingest from the Rift SDK middleware. Authenticated by the
    // tenant's `rl_live_` secret key (or x402). The SDK posts one event per
    // instrumented tool call.
    Router::new()
        .route("/v1/agents/actions", post(routes::record_action))
        .layer(middleware::from_fn_with_state(
            state,
            require_auth(SECRET | X402),
        ))
}
