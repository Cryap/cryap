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
    extract::Path,
    handler::Handler,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use db::{
    models::{Post, User},
    types::DbId,
};
use web::{errors::AppError, AppState};

use crate::{
    activities::UserInbox,
    middleware,
    objects::{note::ApNote, user::ApUser},
};

pub async fn http_get_post(
    Path(id): Path<String>,
    state: Data<Arc<AppState>>,
) -> Result<impl IntoResponse, AppError> {
    println!("{:#?}", id);
    //    let accept = header_map.get("accept").map(|v| v.to_str().unwrap());
    //    if accept == Some(FEDERATION_CONTENT_TYPE) {
    let post = Post::by_id(&DbId::from(id), &state.db_pool).await?;
    if let Some(post) = post {
        let json_post = ApNote(post).into_json(&state).await.unwrap();
        Ok(FederationJson(WithContext::new_default(json_post)).into_response())
    } else {
        Ok(StatusCode::NOT_FOUND.into_response())
    }
    //    } else {
    //        unreachable!()
    //    }
}

pub fn posts() -> Router {
    Router::new().route("/p/:id", get(http_get_post))
}
