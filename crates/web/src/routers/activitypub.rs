use std::sync::Arc;

use activitypub_federation::axum::inbox::{receive_activity, ActivityData};
use activitypub_federation::axum::json::FederationJson;
use activitypub_federation::config::Data;
use activitypub_federation::protocol::context::WithContext;
use activitypub_federation::traits::Object;
use axum::{extract::Path, http::StatusCode, response::IntoResponse};
use db::models::User;

use crate::errors::AppError;
use crate::AppState;

pub async fn http_get_service_actor(
    state: Data<Arc<AppState>>,
) -> Result<impl IntoResponse, AppError> {
    Ok(FederationJson(WithContext::new_default(state.service_actor.clone())).into_response())
}
