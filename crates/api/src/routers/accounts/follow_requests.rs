use std::sync::Arc;

use activitypub_federation::config::Data;
use axum::{
    extract::{Path, Query, State},
    handler::Handler,
    http::{header, StatusCode},
    middleware::from_fn_with_state,
    response::IntoResponse,
    routing::{get, post},
    Extension, Json, Router,
};
use db::{
    models::{user_follow_request::UserFollowRequest, PrivateNote, Session, User},
    pagination::PaginationQuery,
    types::DbId,
};
use web::{errors::AppError, AppState};

use crate::{
    common::follows,
    entities::{Account, Relationship},
    error::ApiError,
    routers::accounts::auth_middleware,
};

// https://docs.joinmastodon.org/methods/follow_requests/#get
pub async fn http_get_follow_requests(
    state: State<Arc<AppState>>,
    Extension(session): Extension<Session>,
    Query(pagination): Query<PaginationQuery>,
) -> Result<impl IntoResponse, AppError> {
    let accounts = Account::new_from_vec(
        UserFollowRequest::by_user(&session.user_id, pagination.into(), &state.db_pool).await?,
    );

    if accounts.is_empty() {
        Ok(Json(accounts).into_response())
    } else {
        Ok((
            [(
                header::LINK, format!(
                    "<https://{}/api/v1/follow_requests?max_id={}>; rel=\"next\", <https://{}/api/v1/follow_requests?min_id={}>; rel\"prev\"",
                    state.config.web.domain, accounts.last().unwrap().id.clone(),
                    state.config.web.domain, accounts.first().unwrap().id.clone()
                )
            )],
            Json(accounts),
        ).into_response())
    }
}

// https://docs.joinmastodon.org/methods/follow_requests/#accept
pub async fn http_post_authorize(
    state: Data<Arc<AppState>>,
    Path(id): Path<String>,
    Extension(session): Extension<Session>,
) -> Result<impl IntoResponse, AppError> {
    let id = DbId::from(id);

    let by = User::by_id(&id, &state.db_pool).await?;
    let to = session.user(&state.db_pool).await?;

    if let Some(by) = by {
        let request = UserFollowRequest::by_actor_and_follower(&by, &to, &state.db_pool).await?;
        if let Some(request) = request {
            follows::accept_follow_request(&by, &to, request.ap_id.unwrap_or_default(), &state)
                .await?;
            Ok(Json(Relationship {
                id: by.id.to_string(),
                following: to.follows(&by, &state.db_pool).await?,
                followed_by: true,
                requested: false,
                note: PrivateNote::get(&to, &by, &state.db_pool)
                    .await?
                    .unwrap_or_default(),
            })
            .into_response())
        } else {
            Ok(ApiError::new("Record not found", StatusCode::NOT_FOUND).into_response())
        }
    } else {
        Ok(ApiError::new("Record not found", StatusCode::NOT_FOUND).into_response())
    }
}

// https://docs.joinmastodon.org/methods/follow_requests/#reject
pub async fn http_post_reject(
    state: Data<Arc<AppState>>,
    Path(id): Path<String>,
    Extension(session): Extension<Session>,
) -> Result<impl IntoResponse, AppError> {
    let id = DbId::from(id);

    let by = User::by_id(&id, &state.db_pool).await?;
    let to = session.user(&state.db_pool).await?;

    if let Some(by) = by {
        let request = UserFollowRequest::by_actor_and_follower(&by, &to, &state.db_pool).await?;
        if let Some(request) = request {
            follows::remove_from_followers(
                &to,
                &by,
                Some(request.ap_id.unwrap_or_default()),
                &state,
            )
            .await?;
            Ok(Json(Relationship {
                id: by.id.to_string(),
                following: to.follows(&by, &state.db_pool).await?,
                followed_by: false,
                requested: false,
                note: PrivateNote::get(&to, &by, &state.db_pool)
                    .await?
                    .unwrap_or_default(),
            })
            .into_response())
        } else {
            Ok(ApiError::new("Record not found", StatusCode::NOT_FOUND).into_response())
        }
    } else {
        Ok(ApiError::new("Record not found", StatusCode::NOT_FOUND).into_response())
    }
}

pub fn follow_requests(state: &Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/v1/follow_requests",
            get(http_get_follow_requests
                .layer(from_fn_with_state(Arc::clone(state), auth_middleware))),
        )
        .route(
            "/api/v1/follow_requests/:id/authorize",
            post(http_post_authorize.layer(from_fn_with_state(Arc::clone(state), auth_middleware))),
        )
        .route(
            "/api/v1/follow_requests/:id/reject",
            post(http_post_reject.layer(from_fn_with_state(Arc::clone(state), auth_middleware))),
        )
}
