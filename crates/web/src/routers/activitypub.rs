use std::sync::Arc;

use activitypub_federation::{
    axum::json::FederationJson, config::Data, protocol::context::WithContext,
};
use axum::response::IntoResponse;

use crate::{errors::AppError, AppState};

pub async fn http_get_service_actor(
    state: Data<Arc<AppState>>,
) -> Result<impl IntoResponse, AppError> {
    Ok(FederationJson(WithContext::new_default(state.service_actor.clone())).into_response())
}
