pub mod routes;

use axum::middleware;
use axum::routing::{delete, get, post};
use axum::Router;
use std::sync::Arc;

use super::auth::middleware::{require_auth, ANONYMOUS, SECRET, X402};
use crate::app::AppState;

pub fn router(state: Arc<AppState>) -> Router<Arc<AppState>> {
    // Authenticated routes (app management).
    let authenticated = Router::new()
        .route("/v1/apps", post(routes::create_app))
        .route("/v1/apps", get(routes::list_apps))
        .route("/v1/apps/{app_id}", delete(routes::delete_app))
        .layer(middleware::from_fn_with_state(
            state,
            require_auth(SECRET | X402 | ANONYMOUS),
        ));

    // Public routes (association files served via custom domain).
    let public = Router::new()
        .route(
            "/.well-known/apple-app-site-association",
            get(routes::serve_aasa),
        )
        .route(
            "/.well-known/assetlinks.json",
            get(routes::serve_assetlinks),
        );

    Router::new().merge(authenticated).merge(public)
}
