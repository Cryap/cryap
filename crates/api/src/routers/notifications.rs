use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    handler::Handler,
    http::{header, StatusCode},
    middleware::from_fn_with_state,
    response::IntoResponse,
    routing::{get, post},
    Extension, Json, Router,
};
use axum_extra::extract::Query as QueryExtra;
use db::{
    models::{Notification, Session},
    pagination::PaginationQuery,
    types::{DbId, DbNotificationType},
};
use serde::Deserialize;
use web::{errors::AppError, AppState};

use crate::{
    auth_middleware::auth_middleware, entities::Notification as ApiNotification, error::ApiError,
    EmptyJsonObject,
};

#[derive(Deserialize)]
pub struct GetQuery {
    #[serde(default, rename = "types[]")]
    types: Vec<String>,
    #[serde(default, rename = "exclude_types[]")]
    exclude_types: Vec<String>,
    account_id: Option<String>,
}

// https://docs.joinmastodon.org/methods/notifications/#get
pub async fn http_get_get(
    state: State<Arc<AppState>>,
    Extension(session): Extension<Session>,
    Query(pagination): Query<PaginationQuery>,
    QueryExtra(query): QueryExtra<GetQuery>,
) -> Result<impl IntoResponse, AppError> {
    let notifications = ApiNotification::build_from_vec(
        Notification::get_for_user(
            &session.user_id,
            pagination.into(),
            if query.types.is_empty() {
                None
            } else {
                Some(
                    query
                        .types
                        .into_iter()
                        .filter_map(|string| DbNotificationType::from_string(&string))
                        .collect(),
                )
            },
            if query.exclude_types.is_empty() {
                None
            } else {
                Some(
                    query
                        .exclude_types
                        .into_iter()
                        .filter_map(|string| DbNotificationType::from_string(&string))
                        .collect(),
                )
            },
            query.account_id.map(DbId::from),
            &state.db_pool,
        )
        .await?,
        &state,
    )
    .await?;

    if notifications.is_empty() {
        Ok(Json(notifications).into_response())
    } else {
        Ok((
            [(
                header::LINK, format!(
                    "<https://{}/api/v1/notifications?max_id={}>; rel=\"next\", <https://{}/api/v1/notifications?min_id={}>; rel\"prev\"",
                    state.config.web.domain, notifications.last().unwrap().id.clone(),
                    state.config.web.domain, notifications.first().unwrap().id.clone()
                )
            )],
            Json(notifications),
        ).into_response())
    }
}

// https://docs.joinmastodon.org/methods/notifications/#get-one
pub async fn http_get_get_one(
    state: State<Arc<AppState>>,
    Extension(session): Extension<Session>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let id = DbId::from(id);
    let notification = Notification::by_id(&id, &state.db_pool).await?;
    match notification {
        Some(notification) if notification.receiver_id == session.user_id => {
            Ok(Json(ApiNotification::build(notification, &state).await?).into_response())
        },
        _ => Ok(ApiError::new("Record not found", StatusCode::NOT_FOUND).into_response()),
    }
}

// https://docs.joinmastodon.org/methods/notifications/#clear
pub async fn http_post_clear(
    state: State<Arc<AppState>>,
    Extension(session): Extension<Session>,
) -> Result<impl IntoResponse, AppError> {
    Notification::clear_for_user(&session.user_id, &state.db_pool).await?;
    Ok(EmptyJsonObject::response())
}

// https://docs.joinmastodon.org/methods/notifications/#dismiss
pub async fn http_post_dismiss(
    state: State<Arc<AppState>>,
    Extension(session): Extension<Session>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let id = DbId::from(id);
    let notification = Notification::by_id(&id, &state.db_pool).await?;
    match notification {
        Some(notification) if notification.receiver_id == session.user_id => {
            notification.dismiss(&state.db_pool).await?;
            Ok(EmptyJsonObject::response())
        },
        _ => Ok(ApiError::new("Record not found", StatusCode::NOT_FOUND).into_response()),
    }
}

pub fn notifications(state: &Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/v1/notifications",
            get(http_get_get.layer(from_fn_with_state(Arc::clone(state), auth_middleware))),
        )
        .route(
            "/api/v1/notifications/:id",
            get(http_get_get_one.layer(from_fn_with_state(Arc::clone(state), auth_middleware))),
        )
        .route(
            "/api/v1/notifications/clear",
            post(http_post_clear.layer(from_fn_with_state(Arc::clone(state), auth_middleware))),
        )
        .route(
            "/api/v1/notifications/:id/dismiss",
            post(http_post_dismiss.layer(from_fn_with_state(Arc::clone(state), auth_middleware))),
        )
}
