use std::sync::Arc;

use activitypub_federation::{
    axum::{
        inbox::{receive_activity, ActivityData},
        json::FederationJson,
    },
    config::Data,
    protocol::context::WithContext,
    traits::Object,
};
use axum::{
    body::Body,
    extract::{Path, Query},
    handler::Handler,
    http::{Request, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use db::{common::timelines::TimelineEntry, models::User, pagination::PaginationQuery};
use futures::future::try_join_all;
use serde::{Deserialize, Serialize};
use web::{errors::AppError, AppState};

use crate::{
    activities::{announce::Announce, Inbox},
    middleware,
    objects::{
        announce::ApAnnounce,
        note::{ApNote, Note},
        ordered_collection::{OrderedCollection, OrderedCollectionPage},
        user::ApUser,
    },
};

pub async fn http_post_shared_inbox(
    state: Data<Arc<AppState>>,
    activity_data: ActivityData,
) -> Result<impl IntoResponse, AppError> {
    Ok(
        receive_activity::<WithContext<Inbox>, ApUser, Arc<AppState>>(activity_data, &state)
            .await?,
    )
}

pub async fn http_post_user_inbox(
    state: Data<Arc<AppState>>,
    Path(name): Path<String>,
    activity_data: ActivityData,
) -> Result<impl IntoResponse, AppError> {
    let user = User::local_by_name(&name, &state.db_pool).await?;
    if let Some(_) = user {
        Ok(
            receive_activity::<WithContext<Inbox>, ApUser, Arc<AppState>>(activity_data, &state)
                .await?
                .into_response(),
        )
    } else {
        Ok(StatusCode::NOT_FOUND.into_response())
    }
}

#[derive(Deserialize)]
pub struct OutboxQuery {
    page: Option<bool>,
}

#[derive(Serialize)]
pub enum OutboxItem {
    Note(Note),
    Announce(Announce),
}

pub async fn http_get_user_outbox(
    state: Data<Arc<AppState>>,
    Path(name): Path<String>,
    Query(query): Query<OutboxQuery>,
    Query(pagination): Query<PaginationQuery>,
    request: Request<Body>,
) -> Result<impl IntoResponse, AppError> {
    let user = User::local_by_name(&name, &state.db_pool).await?;
    if let Some(user) = user {
        if let Some(true) = query.page {
            let timeline = user
                .posts(pagination.into(), None, false, false, &state.db_pool)
                .await?;
            let items = try_join_all(timeline.clone().into_iter().map(|entry| async {
                Result::<OutboxItem, anyhow::Error>::Ok(match entry {
                    TimelineEntry::Post(post) => {
                        OutboxItem::Note(ApNote(post).into_json(&state).await?)
                    },
                    TimelineEntry::Boost(boost, _) => {
                        OutboxItem::Announce(ApAnnounce(boost).into_json(&state).await?)
                    },
                })
            }))
            .await?;
            Ok(
                FederationJson(WithContext::new_default(OrderedCollectionPage::<
                    OutboxItem,
                > {
                    kind: Default::default(),
                    id: format!(
                        "https://{}/u/{}/ap/outbox{}",
                        state.config.web.domain,
                        user.name,
                        request.uri().query().unwrap_or("")
                    ),
                    total_items: user.posts_count,
                    next: if let Some(last) = timeline.last() {
                        Some(format!(
                            "https://{}/u/{}/ap/outbox?max_id={}&page=true",
                            state.config.web.domain,
                            user.name,
                            match last {
                                TimelineEntry::Post(post) => &post.id,
                                TimelineEntry::Boost(boost, _) => &boost.id,
                            }
                        ))
                    } else {
                        None
                    },
                    prev: if let Some(first) = timeline.first() {
                        Some(format!(
                            "https://{}/u/{}/ap/outbox?min_id={}&page=true",
                            state.config.web.domain,
                            user.name,
                            match first {
                                TimelineEntry::Post(post) => &post.id,
                                TimelineEntry::Boost(boost, _) => &boost.id,
                            }
                        ))
                    } else {
                        None
                    },
                    part_of: format!(
                        "https://{}/u/{}/ap/outbox",
                        state.config.web.domain, user.name
                    ),
                    ordered_items: items,
                }))
                .into_response(),
            )
        } else {
            Ok(FederationJson(WithContext::new_default(OrderedCollection {
                kind: Default::default(),
                id: format!(
                    "https://{}/u/{}/ap/outbox",
                    state.config.web.domain, user.name
                ),
                total_items: user.posts_count,
                first: format!(
                    "https://{}/u/{}/ap/outbox?page=true",
                    state.config.web.domain, user.name
                ),
            }))
            .into_response())
        }
    } else {
        Ok(StatusCode::NOT_FOUND.into_response())
    }
}

pub async fn http_get_user(
    //    header_map: HeaderMap,
    Path(name): Path<String>,
    state: Data<Arc<AppState>>,
) -> Result<impl IntoResponse, AppError> {
    //    let accept = header_map.get("accept").map(|v| v.to_str().unwrap());
    //    if accept == Some(FEDERATION_CONTENT_TYPE) {
    let user = User::local_by_name(&name, &state.db_pool).await?;
    if let Some(user) = user {
        let json_user = ApUser(user).into_json(&state).await.unwrap();
        Ok(FederationJson(WithContext::new_default(json_user)).into_response())
    } else {
        Ok(StatusCode::NOT_FOUND.into_response())
    }
    //    } else {
    //        unreachable!()
    //    }
}

pub fn users() -> Router {
    Router::new()
        .route(
            "/ap/inbox",
            post(http_post_shared_inbox.layer(axum::middleware::from_fn(middleware::print_inbox))),
        )
        .route(
            "/u/:name/ap/inbox",
            post(http_post_user_inbox.layer(axum::middleware::from_fn(middleware::print_inbox))),
        )
        .route("/u/:name/ap/outbox", get(http_get_user_outbox))
        .route("/u/:name", get(http_get_user))
}
