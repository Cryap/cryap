pub mod accounts;
pub mod apps;
pub mod auth;
pub mod instance;
pub mod statuses;
pub mod timelines;

use std::sync::Arc;

use axum::Router;
use web::AppState;

pub fn api(state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .merge(accounts::accounts(&state))
        .merge(statuses::statuses(&state))
        .merge(apps::apps(&state))
        .merge(timelines::timelines(&state))
        .merge(auth::auth())
        .merge(instance::instance())
}
