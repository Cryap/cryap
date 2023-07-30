use std::sync::Arc;

use axum::{extract::State, response::IntoResponse, routing::get, Json, Router};
use web::{errors::AppError, AppState};

use crate::entities::{instance_v1, instance_v2, Rule};

// https://docs.joinmastodon.org/methods/instance/#v2
pub async fn http_get_instance_v2(
    state: State<Arc<AppState>>,
) -> Result<impl IntoResponse, AppError> {
    Ok(Json(instance_v2::Instance::new(&state.config)).into_response())
}

// https://docs.joinmastodon.org/methods/instance/#v1
pub async fn http_get_instance_v1(
    state: State<Arc<AppState>>,
) -> Result<impl IntoResponse, AppError> {
    Ok(Json(instance_v1::Instance::new(&state.config)).into_response())
}

// https://docs.joinmastodon.org/methods/instance/#rules
pub async fn http_get_instance_rules(
    state: State<Arc<AppState>>,
) -> Result<impl IntoResponse, AppError> {
    Ok(Json(
        state
            .config
            .instance
            .rules
            .clone()
            .into_iter()
            .enumerate()
            .map(|(index, rule)| Rule {
                id: (index + 1).to_string(),
                text: rule,
            })
            .collect::<Vec<Rule>>(),
    )
    .into_response())
}

pub fn instance() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/v2/instance", get(http_get_instance_v2))
        .route("/api/v1/instance", get(http_get_instance_v1))
        .route("/api/v1/instance/rules", get(http_get_instance_rules))
}
