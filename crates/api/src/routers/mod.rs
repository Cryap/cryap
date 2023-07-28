pub mod accounts;
pub mod apps;
pub mod auth;

use std::sync::Arc;

use axum::Router;
use web::AppState;

pub fn api(state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .merge(accounts::accounts(&state))
        .merge(apps::apps(&state))
        .merge(auth::auth())
}
