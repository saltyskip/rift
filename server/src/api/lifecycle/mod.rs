pub mod routes;

use axum::middleware;
use axum::routing::{post, put};
use axum::Router;
use std::sync::Arc;

use super::auth::middleware::sdk_auth_gate;
use crate::app::AppState;

pub fn router(state: Arc<AppState>) -> Router<Arc<AppState>> {
    // SDK-authenticated lifecycle endpoints — pk_live_ sdk_auth_gate.
    //
    // The three SDK calls in user-funnel order: `click` (server-side via the
    // Web SDK), `attribute` (link touched in app), `identify` (install bound
    // to user_id). Conversion tracking is `POST /v1/lifecycle/convert` and
    // lives in the conversions slice.
    Router::new()
        .route("/v1/lifecycle/click", post(routes::lifecycle_click))
        .route("/v1/lifecycle/attribute", post(routes::lifecycle_attribute))
        .route("/v1/lifecycle/identify", put(routes::lifecycle_identify))
        .layer(middleware::from_fn_with_state(state, sdk_auth_gate))
}
