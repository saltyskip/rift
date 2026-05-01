pub mod middleware;
pub mod models;
pub mod publishable_keys;
pub mod secret_keys;
pub mod users;

use axum::Router;
use std::sync::Arc;

use crate::app::AppState;

pub fn router(state: Arc<AppState>) -> Router<Arc<AppState>> {
    secret_keys::router(state.clone())
        .merge(publishable_keys::router(state.clone()))
        .merge(users::router(state))
}
