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
    models::{Post, PostBoost, Session},
    types::{DbId, DbVisibility},
};
use serde::Deserialize;
use web::{errors::AppError, AppState};

use crate::{auth_middleware::auth_middleware, common::posts, entities::Status, error::ApiError};

// https://docs.joinmastodon.org/methods/statuses/#get
pub async fn http_get_get(
    state: State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let id = DbId::from(id);
    let (post, boost) = posts::post_or_boost_by_id(&id, &state.db_pool).await?;
    if let Some(post) = post {
        match boost {
            Some(boost) => {
                Ok(Json(Status::build_from_boost(boost, None, &state).await?).into_response())
            }
            None => Ok(Json(Status::build(post, None, &state).await?).into_response()),
        }
    } else {
        Ok(ApiError::new("Record not found", StatusCode::NOT_FOUND).into_response())
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
    let (post, boost) = posts::post_or_boost_by_id(&id, &state.db_pool).await?;

    if let Some(post) = post {
        if !post.liked_by(&user, &state.db_pool).await? {
            posts::like(&user, &post, &state).await?;
        }

        match boost {
            Some(boost) => {
                Ok(Json(Status::build_from_boost(boost, None, &state).await?).into_response())
            }
            None => Ok(Json(Status::build(post, None, &state).await?).into_response()),
        }
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
    let (post, boost) = posts::post_or_boost_by_id(&id, &state.db_pool).await?;

    if let Some(post) = post {
        if post.liked_by(&user, &state.db_pool).await? {
            posts::unlike(&user, &post, &state).await?;
        }

        match boost {
            Some(boost) => {
                Ok(Json(Status::build_from_boost(boost, None, &state).await?).into_response())
            }
            None => Ok(Json(Status::build(post, None, &state).await?).into_response()),
        }
    } else {
        Ok(ApiError::new("Record not found", StatusCode::NOT_FOUND).into_response())
    }
}

#[derive(Deserialize)]
pub struct ReblogBody {
    visibility: Option<String>,
}

// https://docs.joinmastodon.org/methods/statuses/#boost
pub async fn http_post_reblog(
    state: Data<Arc<AppState>>,
    Path(id): Path<String>,
    Extension(session): Extension<Session>,
    body: Option<Json<ReblogBody>>,
) -> Result<impl IntoResponse, AppError> {
    let id = DbId::from(id);

    let user = session.user(&state.db_pool).await?;
    let (post, _) = posts::post_or_boost_by_id(&id, &state.db_pool).await?;

    if let Some(post) = post {
        if post.visibility != DbVisibility::Public && post.visibility != DbVisibility::Unlisted {
            return Ok(
                ApiError::new("This action is not allowed", StatusCode::FORBIDDEN).into_response(),
            );
        }

        let visibility = body
            .as_ref()
            .and_then(|body| body.visibility.as_ref())
            .and_then(|visibility| DbVisibility::from_string(visibility))
            .unwrap_or(DbVisibility::Public); // TODO: Add setting
        if visibility == DbVisibility::Direct {
            return Ok(ApiError::new(
                "Validation failed: Visibility is reserved",
                StatusCode::UNPROCESSABLE_ENTITY,
            )
            .into_response());
        }

        let boost = if let Some(boost) = post.boost_by(&user, &state.db_pool).await? {
            boost
        } else {
            posts::boost(&user, &post, visibility, &state).await?
        };

        Ok(Json(Status::build_from_boost(boost, None, &state).await?).into_response())
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
        .route(
            "/api/v1/statuses/:id/reblog",
            post(http_post_reblog.layer(from_fn_with_state(Arc::clone(state), auth_middleware))),
        )
}
