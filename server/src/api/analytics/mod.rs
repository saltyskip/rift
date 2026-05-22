//! HTTP transport for the analytics surface (funnel stats today;
//! timeseries / cohorts later land here too).

pub mod models;
pub mod routes;

use axum::middleware;
use axum::routing::get;
use axum::Router;
use std::sync::Arc;

use super::auth::middleware::auth_gate;
use crate::app::AppState;

pub fn router(state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .route("/v1/analytics/stats", get(routes::get_stats))
        .layer(middleware::from_fn_with_state(state, auth_gate))
}
