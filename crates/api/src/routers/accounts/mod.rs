mod bookmarks;
mod favourites;

use std::sync::Arc;

use activitypub_federation::config::Data;
use axum::{
    extract::{Extension, Path, Query, State},
    handler::Handler,
    http::{header, StatusCode},
    middleware::from_fn_with_state,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use axum_extra::extract::Query as QueryExtra;
use db::{
    models::{PrivateNote, Session, User},
    pagination::PaginationQuery,
    types::DbId,
};
use futures::future::join_all;
use serde::Deserialize;
use web::{errors::AppError, AppState};

use crate::{
    auth_middleware::auth_middleware,
    common::follows,
    entities::{Account, Relationship},
    error::ApiError,
};

// TODO: Fully implement https://docs.joinmastodon.org/methods/accounts/#verify_credentials
pub async fn http_get_verify_credentials(
    state: State<Arc<AppState>>,
    Extension(session): Extension<Session>,
) -> Result<impl IntoResponse, AppError> {
    Ok(
        Json(Account::build(session.user(&state.db_pool).await?, &state, true).await?)
            .into_response(),
    )
}

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
        Some(user) => Ok(Json(Account::build(user, &state, false).await?).into_response()),
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
        Some(user) => Ok(Json(Account::build(user, &state, false).await?).into_response()),
        None => Ok(ApiError::new("Record not found", StatusCode::NOT_FOUND).into_response()),
    }
}

// https://docs.joinmastodon.org/methods/accounts/#followers
pub async fn http_get_followers(
    state: Data<Arc<AppState>>,
    Path(id): Path<String>,
    Query(pagination): Query<PaginationQuery>,
) -> Result<impl IntoResponse, AppError> {
    let id = DbId::from(id);
    let user = User::by_id(&id, &state.db_pool).await?;

    if let Some(user) = user {
        let accounts = Account::build_from_vec(
            user.followers(pagination.into(), &state.db_pool).await?,
            &state,
        )
        .await?;

        if accounts.is_empty() {
            Ok(Json(accounts).into_response())
        } else {
            Ok((
                [(
                    header::LINK, format!(
                        "<https://{}/api/v1/accounts/{}/followers?max_id={}>; rel=\"next\", <https://{}/api/v1/accounts/{}/followers?min_id={}>; rel\"prev\"",
                        state.config.web.domain, id, accounts.last().unwrap().id.clone(),
                        state.config.web.domain, id, accounts.first().unwrap().id.clone()
                    )
                )],
                Json(accounts),
            ).into_response())
        }
    } else {
        Ok(ApiError::new("Record not found", StatusCode::NOT_FOUND).into_response())
    }
}

// https://docs.joinmastodon.org/methods/accounts/#following
pub async fn http_get_following(
    state: Data<Arc<AppState>>,
    Path(id): Path<String>,
    Query(pagination): Query<PaginationQuery>,
) -> Result<impl IntoResponse, AppError> {
    let id = DbId::from(id);
    let user = User::by_id(&id, &state.db_pool).await?;

    if let Some(user) = user {
        let accounts = Account::build_from_vec(
            user.following(pagination.into(), &state.db_pool).await?,
            &state,
        )
        .await?;

        if accounts.is_empty() {
            Ok(Json(accounts).into_response())
        } else {
            Ok((
                [(
                    header::LINK, format!(
                        "<https://{}/api/v1/accounts/{}/followers?max_id={}>; rel=\"next\", <https://{}/api/v1/accounts/{}/followers?min_id={}>; rel\"prev\"",
                        state.config.web.domain, id, accounts.last().unwrap().id.clone(),
                        state.config.web.domain, id, accounts.first().unwrap().id.clone()
                    )
                )],
                Json(accounts),
            ).into_response())
        }
    } else {
        Ok(ApiError::new("Record not found", StatusCode::NOT_FOUND).into_response())
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

#[derive(Deserialize)]
pub struct NoteBody {
    comment: String,
}

// https://docs.joinmastodon.org/methods/accounts/#note
pub async fn http_post_note(
    state: Data<Arc<AppState>>,
    Path(id): Path<String>,
    Extension(session): Extension<Session>,
    Json(body): Json<NoteBody>,
) -> Result<impl IntoResponse, AppError> {
    let id = DbId::from(id);

    let by = session.user(&state.db_pool).await?;
    let to = User::by_id(&id, &state.db_pool).await?;

    if let Some(to) = to {
        let note = if body.comment.trim().is_empty() {
            PrivateNote::set(&by, &to, None, &state.db_pool).await?;
            String::new()
        } else {
            if body.comment.len() > 2000 {
                return Ok(ApiError::new(
                    "Validation failed: Comment is too long (maximum is 2000 characters)",
                    StatusCode::UNPROCESSABLE_ENTITY,
                )
                .into_response());
            }

            PrivateNote::set(&by, &to, Some(&body.comment), &state.db_pool).await?;
            body.comment
        };

        Ok(Json(Relationship {
            id: to.id.to_string(),
            following: by.follows(&to, &state.db_pool).await?,
            followed_by: to.follows(&by, &state.db_pool).await?,
            requested: by.wants_to_follow(&to, &state.db_pool).await?,
            note,
        })
        .into_response())
    } else {
        Ok(ApiError::new("Record not found", StatusCode::NOT_FOUND).into_response())
    }
}

// This can be done using enum, but https://github.com/nox/serde_urlencoded/issues/66
#[derive(Deserialize)]
pub struct RelationshipsQuery {
    id: Option<String>,

    #[serde(rename = "id[]")]
    ids: Option<Vec<String>>,
}

// https://docs.joinmastodon.org/methods/accounts/#relationships
pub async fn http_get_relationships(
    state: State<Arc<AppState>>,
    QueryExtra(query): QueryExtra<RelationshipsQuery>,
    Extension(session): Extension<Session>,
) -> Result<impl IntoResponse, AppError> {
    let ids = if let Some(id) = query.id {
        vec![id]
    } else if let Some(ids) = query.ids {
        ids
    } else {
        vec![]
    };

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
        .flatten()
        .collect::<Vec<Relationship>>(),
    ))
}

pub fn accounts(state: &Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .merge(bookmarks::bookmarks(state))
        .merge(favourites::favourites(state))
        .route(
            "/api/v1/accounts/verify_credentials",
            get(http_get_verify_credentials
                .layer(from_fn_with_state(Arc::clone(state), auth_middleware))),
        )
        .route("/api/v1/accounts/lookup", get(http_get_lookup))
        .route("/api/v1/accounts/:id", get(http_get_get))
        .route("/api/v1/accounts/:id/followers", get(http_get_followers))
        .route("/api/v1/accounts/:id/following", get(http_get_following))
        .route(
            "/api/v1/accounts/:id/follow",
            post(http_post_follow.layer(from_fn_with_state(Arc::clone(state), auth_middleware))),
        )
        .route(
            "/api/v1/accounts/:id/unfollow",
            post(http_post_unfollow.layer(from_fn_with_state(Arc::clone(state), auth_middleware))),
        )
        .route(
            "/api/v1/accounts/:id/remove_from_followers",
            post(
                http_post_remove_from_followers
                    .layer(from_fn_with_state(Arc::clone(state), auth_middleware)),
            ),
        )
        .route(
            "/api/v1/accounts/:id/note",
            post(http_post_note.layer(from_fn_with_state(Arc::clone(state), auth_middleware))),
        )
        .route(
            "/api/v1/accounts/relationships",
            get(http_get_relationships
                .layer(from_fn_with_state(Arc::clone(state), auth_middleware))),
        )
}
