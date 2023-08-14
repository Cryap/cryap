use std::sync::Arc;

use activitypub_federation::config::Data;
use axum::{
    extract::{Path, State},
    handler::Handler,
    http::StatusCode,
    middleware::from_fn_with_state,
    response::IntoResponse,
    routing::{get, post},
    Extension, Json, Router,
};
use db::{
    models::{Post, Session},
    types::{DbId, DbVisibility},
};
use serde::Deserialize;
use web::{errors::AppError, AppState};

use crate::{
    auth_middleware::auth_middleware,
    common::{self, posts},
    entities::Status,
    error::ApiError,
};

// https://docs.joinmastodon.org/methods/statuses/#get
pub async fn http_get_home(
    state: State<Arc<AppState>>,
    Extension(session): Extension<Session>,
) -> Result<impl IntoResponse, AppError> {
    Ok(Json::<Vec<()>>(vec![]))
}

pub fn timelines(state: &Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/v1/timelines/home",
            get(http_get_home.layer(from_fn_with_state(Arc::clone(state), auth_middleware))),
        )
}
