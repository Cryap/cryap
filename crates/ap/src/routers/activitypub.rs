use activitypub_federation::{axum::json::FederationJson, protocol::context::WithContext};
use axum::{handler::Handler, response::IntoResponse, routing::get, Extension, Router};
use web::errors::AppError;

use crate::objects::service_actor::ServiceActor;

pub async fn http_get_service_actor(
    Extension(service_actor): Extension<ServiceActor>,
) -> Result<impl IntoResponse, AppError> {
    Ok(FederationJson(WithContext::new_default(service_actor.clone())).into_response())
}

pub fn activitypub(service_actor: ServiceActor) -> Router {
    Router::new().route(
        "/ap/actor",
        get(http_get_service_actor.layer(Extension(service_actor))),
    )
}
