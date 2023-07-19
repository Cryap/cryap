use std::sync::Arc;

use activitypub_federation::axum::inbox::{receive_activity, ActivityData};
use activitypub_federation::axum::json::FederationJson;
use activitypub_federation::config::Data;
use activitypub_federation::protocol::context::WithContext;
use activitypub_federation::traits::Object;
use axum::{extract::Path, http::StatusCode, response::IntoResponse};
use db::models::User;

use crate::ap::activities::UserInbox;
use crate::ap::objects::user::ApUser;
use crate::errors::AppError;
use crate::AppState;

pub async fn http_post_user_inbox(
    state: Data<Arc<AppState>>,
    activity_data: ActivityData,
) -> Result<impl IntoResponse, AppError> {
    Ok(
        receive_activity::<WithContext<UserInbox>, ApUser, Arc<AppState>>(activity_data, &state)
            .await?,
    )
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
