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
    types::DbId,
};
use web::{errors::AppError, AppState};

use crate::{auth_middleware::auth_middleware, common::posts, entities::Status, error::ApiError};

// https://docs.joinmastodon.org/methods/statuses/#get
pub async fn http_get_get(
    state: State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let id = DbId::from(id);
    let post = Post::by_id(&id, &state.db_pool).await?;
    match post {
        Some(post) => Ok(Json(Status::build(post, None, &state).await?).into_response()),
        None => Ok(ApiError::new("Record not found", StatusCode::NOT_FOUND).into_response()),
    }
}

// https://docs.joinmastodon.org/methods/statuses/#favourite
pub async fn http_post_favourite(
    state: Data<Arc<AppState>>,
    Path(id): Path<String>,
    Extension(session): Extension<Session>,
) -> Result<impl IntoResponse, AppError> {
    let id = DbId::from(id);

    let user = session.user(&state.db_pool).await?;
    let post = Post::by_id(&id, &state.db_pool).await?;

    if let Some(post) = post {
        if !post.liked_by(&user, &state.db_pool).await? {
            posts::like(&user, &post, &state).await?;
        }

        Ok(Json(Status::build(post, None, &state).await?).into_response())
    } else {
        Ok(ApiError::new("Record not found", StatusCode::NOT_FOUND).into_response())
    }
}

// https://docs.joinmastodon.org/methods/statuses/#unfavourite
pub async fn http_post_unfavourite(
    state: Data<Arc<AppState>>,
    Path(id): Path<String>,
    Extension(session): Extension<Session>,
) -> Result<impl IntoResponse, AppError> {
    let id = DbId::from(id);

    let user = session.user(&state.db_pool).await?;
    let post = Post::by_id(&id, &state.db_pool).await?;

    if let Some(post) = post {
        if post.liked_by(&user, &state.db_pool).await? {
            posts::unlike(&user, &post, &state).await?;
        }

        Ok(Json(Status::build(post, None, &state).await?).into_response())
    } else {
        Ok(ApiError::new("Record not found", StatusCode::NOT_FOUND).into_response())
    }
}

pub fn statuses(state: &Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/v1/statuses/:id", get(http_get_get))
        .route(
            "/api/v1/statuses/:id/favourite",
            post(http_post_favourite.layer(from_fn_with_state(Arc::clone(state), auth_middleware))),
        )
        .route(
            "/api/v1/statuses/:id/unfavourite",
            post(
                http_post_unfavourite.layer(from_fn_with_state(Arc::clone(state), auth_middleware)),
            ),
        )
}
