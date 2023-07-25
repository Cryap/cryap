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
use axum::{extract::Path, http::StatusCode, response::IntoResponse};
use db::models::User;
use web::{errors::AppError, AppState};

use crate::{activities::UserInbox, objects::user::ApUser};

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
