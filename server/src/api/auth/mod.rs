pub mod middleware;
pub mod publishable_keys;
pub mod secret_keys;

use axum::Router;
use std::sync::Arc;

use crate::app::AppState;

pub fn router(state: Arc<AppState>) -> Router<Arc<AppState>> {
    secret_keys::router().merge(publishable_keys::router(state))
}
