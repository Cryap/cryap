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
    auth_middleware::{auth_middleware, optional_auth_middleware},
    common::{self, posts},
    entities::Status,
    error::ApiError,
};

#[derive(Deserialize)]
pub struct CreatePostBody {
    status: String,
    in_reply_to_id: Option<String>,
    quote_id: Option<String>,
    sensitive: Option<bool>,
    spoiler_text: Option<String>,
    visibility: Option<DbVisibility>,
}

// https://docs.joinmastodon.org/methods/statuses/#create
pub async fn http_post_create(
    state: Data<Arc<AppState>>,
    Extension(session): Extension<Session>,
    Json(body): Json<CreatePostBody>,
) -> Result<impl IntoResponse, AppError> {
    let user = session.user(&state.db_pool).await?;

    let post = common::posts::post(
        &user,
        posts::NewPost {
            visibility: body.visibility.unwrap_or(DbVisibility::Public),
            content: body.status,
            in_reply: match body.in_reply_to_id {
                Some(id) => Post::by_id(&DbId::from(id), &state.db_pool).await?,
                None => None,
            },
            quote: match body.quote_id {
                Some(id) => Post::by_id(&DbId::from(id), &state.db_pool).await?,
                None => None,
            },
            local_only: false,
            sensitive: body.sensitive.unwrap_or(false),
            content_warning: body.spoiler_text,
        },
        &state,
    )
    .await?;

    Ok(Json(Status::build(post, None, &state).await?).into_response())
}

// https://docs.joinmastodon.org/methods/statuses/#get
pub async fn http_get_get(
    state: State<Arc<AppState>>,
    Extension(session): Extension<Option<Session>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let id = DbId::from(id);
    let (post, boost) = posts::post_or_boost_by_id(&id, &state.db_pool).await?;
    let user = match session {
        Some(session) => Some(session.user(&state.db_pool).await?),
        None => None,
    };

    if let Some(post) = post {
        match boost {
            Some(boost) => {
                Ok(Json(Status::build_from_boost(boost, None, &state).await?).into_response())
            },
            None => {
                if posts::accessible_for(&post, user.as_ref(), &state.db_pool).await? {
                    Ok(Json(Status::build(post, None, &state).await?).into_response())
                } else {
                    Ok(ApiError::new("Record not found", StatusCode::NOT_FOUND).into_response())
                }
            },
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
        if boost.is_none() && !posts::accessible_for(&post, Some(&user), &state.db_pool).await? {
            return Ok(ApiError::new("Record not found", StatusCode::NOT_FOUND).into_response());
        }

        if !post.liked_by(&user, &state.db_pool).await? {
            posts::like(&user, &post, &state).await?;
        }

        match boost {
            Some(boost) => {
                Ok(Json(Status::build_from_boost(boost, None, &state).await?).into_response())
            },
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
        if boost.is_none() && !posts::accessible_for(&post, Some(&user), &state.db_pool).await? {
            return Ok(ApiError::new("Record not found", StatusCode::NOT_FOUND).into_response());
        }

        if post.liked_by(&user, &state.db_pool).await? {
            posts::unlike(&user, &post, &state).await?;
        }

        match boost {
            Some(boost) => {
                Ok(Json(Status::build_from_boost(boost, None, &state).await?).into_response())
            },
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
    let (post, boost) = posts::post_or_boost_by_id(&id, &state.db_pool).await?;

    if let Some(post) = post {
        if boost.is_none() && !posts::accessible_for(&post, Some(&user), &state.db_pool).await? {
            return Ok(ApiError::new("Record not found", StatusCode::NOT_FOUND).into_response());
        }

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
        .route(
            "/api/v1/statuses",
            post(http_post_create.layer(from_fn_with_state(Arc::clone(state), auth_middleware))),
        )
        .route(
            "/api/v1/statuses/:id",
            get(http_get_get.layer(from_fn_with_state(
                Arc::clone(state),
                optional_auth_middleware,
            ))),
        )
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
