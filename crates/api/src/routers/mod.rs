pub mod accounts;
pub mod apps;
pub mod instance;
pub mod notifications;
pub mod statuses;
pub mod timelines;
pub mod ui;

use std::sync::Arc;

use axum::Router;
use web::AppState;

pub fn api(state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .merge(accounts::accounts(&state))
        .merge(apps::apps(&state))
        .merge(instance::instance())
        .merge(notifications::notifications(&state))
        .merge(statuses::statuses(&state))
        .merge(timelines::timelines(&state))
        .merge(ui::ui())
}
