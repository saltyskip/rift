pub mod routes;

use axum::middleware;
use axum::routing::{delete, get, post};
use axum::Router;
use std::sync::Arc;

use super::auth::middleware::auth_gate;
use crate::app::AppState;

pub fn router(state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .route("/v1/affiliates", post(routes::create_affiliate))
        .route("/v1/affiliates", get(routes::list_affiliates))
        .route(
            "/v1/affiliates/{affiliate_id}",
            get(routes::get_affiliate)
                .patch(routes::patch_affiliate)
                .delete(routes::delete_affiliate),
        )
        .route(
            "/v1/affiliates/{affiliate_id}/credentials",
            post(routes::create_affiliate_credential).get(routes::list_affiliate_credentials),
        )
        .route(
            "/v1/affiliates/{affiliate_id}/credentials/{key_id}",
            delete(routes::revoke_affiliate_credential),
        )
        .layer(middleware::from_fn_with_state(state, auth_gate))
}
