mod auth;

use std::sync::Arc;

use axum::Router;
use web::AppState;

pub fn ui() -> Router<Arc<AppState>> {
    Router::new().merge(auth::auth())
}
