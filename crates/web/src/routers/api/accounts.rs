use std::sync::Arc;

use activitypub_federation::config::Data;
use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use axum_extra::extract::Query as QueryExtra;
use db::{
    models::{Session, User},
    types::DbId,
};
use futures::future::join_all;
use serde::Deserialize;

use crate::{
    api::entities::{Account, Relationship},
    api::ApiError,
    common::follows,
    errors::AppError,
    AppState,
};

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
        if !by.follows(&to, &state.db_pool).await?
            && !by.wants_to_follow(&to, &state.db_pool).await?
        {
            follows::want_to_follow(&by, &to, &state).await?;
        }

        if to.manually_approves_followers {
            Ok(Json(Relationship {
                id: to.id.to_string(),
                following: false,
                followed_by: to.follows(&by, &state.db_pool).await?,
                requested: true,
                note: String::new(),
            })
            .into_response())
        } else {
            Ok(Json(Relationship {
                id: to.id.to_string(),
                following: true,
                followed_by: to.follows(&by, &state.db_pool).await?,
                requested: false,
                note: String::new(),
            })
            .into_response())
        }
    } else {
        Ok(ApiError::new("Record not found", StatusCode::NOT_FOUND).into_response())
    }
}

// https://docs.joinmastodon.org/methods/accounts/#unfollow
pub async fn http_post_unfollow(
    state: Data<Arc<AppState>>,
    Path(id): Path<String>,
    Extension(session): Extension<Session>,
) -> Result<impl IntoResponse, AppError> {
    let id = DbId::from(id);

    let by = session.user(&state.db_pool).await?;
    let to = User::by_id(&id, &state.db_pool).await?;

    if let Some(to) = to {
        if by.follows(&to, &state.db_pool).await? || by.wants_to_follow(&to, &state.db_pool).await?
        {
            follows::unfollow(&by, &to, &state).await?;
        }

        Ok(Json(Relationship {
            id: to.id.to_string(),
            following: false,
            followed_by: to.follows(&by, &state.db_pool).await?,
            requested: false,
            note: String::new(),
        })
        .into_response())
    } else {
        Ok(ApiError::new("Record not found", StatusCode::NOT_FOUND).into_response())
    }
}

// https://docs.joinmastodon.org/methods/accounts/#remove_from_followers
pub async fn http_post_remove_from_followers(
    state: Data<Arc<AppState>>,
    Path(id): Path<String>,
    Extension(session): Extension<Session>,
) -> Result<impl IntoResponse, AppError> {
    let id = DbId::from(id);

    let by = session.user(&state.db_pool).await?;
    let to = User::by_id(&id, &state.db_pool).await?;

    if let Some(to) = to {
        if to.follows(&by, &state.db_pool).await? || to.wants_to_follow(&by, &state.db_pool).await?
        {
            follows::remove_from_followers(&by, &to, &state).await?;
        }

        Ok(Json(Relationship {
            id: to.id.to_string(),
            following: by.follows(&to, &state.db_pool).await?,
            followed_by: false,
            requested: by.wants_to_follow(&to, &state.db_pool).await?,
            note: String::new(),
        })
        .into_response())
    } else {
        Ok(ApiError::new("Record not found", StatusCode::NOT_FOUND).into_response())
    }
}

// TODO: Make private after `nest` fix
#[derive(Deserialize)]
pub struct RelationshipsQuery {
    #[serde(rename = "id[]")]
    ids: Vec<String>,
}

// https://docs.joinmastodon.org/methods/accounts/#relationships
pub async fn http_get_relationships(
    state: Data<Arc<AppState>>,
    QueryExtra(ids): QueryExtra<RelationshipsQuery>,
    Extension(session): Extension<Session>,
) -> Result<impl IntoResponse, AppError> {
    let ids = ids.ids;
    let session_user = session.user(&state.db_pool).await?;
    Ok(Json(
        join_all(ids.into_iter().map(|id| async {
            let user = User::by_id(&id.into(), &state.db_pool).await.ok()?;
            match user {
                Some(user) => Some(
                    Relationship::build(&session_user, &user, &state.db_pool)
                        .await
                        .ok()?,
                ),
                None => None,
            }
        }))
        .await
        .into_iter()
        .filter_map(|relationship| relationship)
        .collect::<Vec<Relationship>>(),
    ))
}
