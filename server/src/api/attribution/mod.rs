pub mod routes;

use axum::middleware;
use axum::routing::{post, put};
use axum::Router;
use std::sync::Arc;

use super::auth::middleware::sdk_auth_gate;
use crate::app::AppState;

pub fn router(state: Arc<AppState>) -> Router<Arc<AppState>> {
    // SDK-authenticated routes — pk_live_ sdk_auth_gate.
    //
    // `PUT /v1/attribution/identify` lives here because the `install_id`
    // argument is opaque and only lives in the mobile SDK — no shipped flow
    // produces the inputs a secret-key backend would need to call this
    // endpoint. Same for the other two: clients are mobile/web SDKs hitting
    // the public-key auth path, not server-to-server callers.
    Router::new()
        .route("/v1/attribution/click", post(routes::attribution_click))
        .route("/v1/attribution/install", post(routes::attribution_report))
        .route("/v1/attribution/identify", put(routes::link_attribution))
        .layer(middleware::from_fn_with_state(state, sdk_auth_gate))
}
