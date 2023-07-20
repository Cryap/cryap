use std::sync::Arc;

use activitypub_federation::config::Data;
use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use db::{
    models::{Session, User},
    types::DbId,
};
use serde::Deserialize;

use crate::{api::entities::Account, api::ApiError, common::follows, errors::AppError, AppState};

// TODO: Fully implement https://docs.joinmastodon.org/methods/accounts/#verify_credentials
pub async fn http_get_verify_credentials(
    state: State<Arc<AppState>>,
    Extension(session): Extension<Session>,
) -> Result<impl IntoResponse, AppError> {
    Ok(Json(Account::new(session.user(&state.db_pool).await?)).into_response())
}

// TODO: Make private after `nest` fix
#[derive(Deserialize)]
pub struct LookupQuery {
    acct: String,
}

// https://docs.joinmastodon.org/methods/accounts/#lookup
pub async fn http_get_lookup(
    state: State<Arc<AppState>>,
    Query(acct): Query<LookupQuery>,
) -> Result<impl IntoResponse, AppError> {
    let acct = acct.acct;
    let user = User::local_by_name(&acct, &state.db_pool).await?;
    let user = match user {
        Some(user) => Some(user),
        None => User::by_acct(acct, &state.db_pool).await?,
    };

    match user {
        Some(user) => Ok(Json(Account::new(user)).into_response()),
        None => Ok(ApiError::new("Record not found", StatusCode::NOT_FOUND).into_response()),
    }
}

// https://docs.joinmastodon.org/methods/accounts/#get
pub async fn http_get_get(
    state: State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let id = DbId::from(id);
    let user = User::by_id(&id, &state.db_pool).await?;
    match user {
        Some(user) => Ok(Json(Account::new(user)).into_response()),
        None => Ok(ApiError::new("Record not found", StatusCode::NOT_FOUND).into_response()),
    }
}

// TODO: Fully implement https://docs.joinmastodon.org/methods/accounts/#follow
pub async fn http_post_follow(
    state: Data<Arc<AppState>>,
    Path(id): Path<String>,
    Extension(session): Extension<Session>,
) -> Result<impl IntoResponse, AppError> {
    let id = DbId::from(id);

    let by = session.user(&state.db_pool).await?;
    let to = User::by_id(&id, &state.db_pool).await?;

    if let Some(to) = to {
        follows::want_to_follow(by, to, &state).await?;
        Ok(
            ApiError::new("TODO: Return Relationship entity", StatusCode::NOT_FOUND)
                .into_response(),
        )
    } else {
        Ok(ApiError::new("Record not found", StatusCode::NOT_FOUND).into_response())
    }
}
