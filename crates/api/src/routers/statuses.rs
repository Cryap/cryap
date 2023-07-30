use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use db::{models::Post, types::DbId};
use web::{errors::AppError, AppState};

use crate::{entities::Status, error::ApiError};

// https://docs.joinmastodon.org/methods/statuses/#get
pub async fn http_get_get(
    state: State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let id = DbId::from(id);
    let user = Post::by_id(&id, &state.db_pool).await?;
    match user {
        Some(user) => Ok(Json(Status::build(user, None, &state).await?).into_response()),
        None => Ok(ApiError::new("Record not found", StatusCode::NOT_FOUND).into_response()),
    }
}

pub fn statuses(_state: &Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new().route("/api/v1/statuses/:id", get(http_get_get))
}
