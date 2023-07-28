use std::sync::Arc;

use activitypub_federation::{
    axum::json::FederationJson,
    config::Data,
    fetch::webfinger::{extract_webfinger_name, Webfinger, WebfingerLink},
    protocol::context::WithContext,
};
use axum::{
    extract::Query, handler::Handler, response::IntoResponse, routing::get, Extension, Json, Router,
};
use db::{models::User, schema};
use diesel::{ExpressionMethods, QueryDsl};
use diesel_async::RunQueryDsl;
use serde::Deserialize;
use url::Url;
use web::{errors::AppError, AppState};

use crate::objects::service_actor::ServiceActor;

#[derive(Deserialize)]
pub struct WebfingerQuery {
    resource: String,
}

pub async fn http_get_webfinger(
    Query(resource): Query<WebfingerQuery>,
    state: Data<Arc<AppState>>,
) -> Result<impl IntoResponse, AppError> {
    let mut connection = state.db_pool.get().await?;
    let resource = resource.resource;

    let name = extract_webfinger_name(&resource, &state)?;
    let user: User = schema::users::table
        .filter(schema::users::local.eq(true))
        .filter(schema::users::name.eq(name.clone()))
        .first(&mut connection)
        .await?;
    Ok(Json(Webfinger {
        subject: resource,
        links: vec![WebfingerLink {
            rel: Some("self".to_string()),
            kind: Some(activitypub_federation::FEDERATION_CONTENT_TYPE.to_string()),
            href: Some(Url::parse(&user.ap_id)?),
            ..Default::default()
        }],
        aliases: vec![Url::parse(&user.ap_id)?],
        properties: Default::default(),
    }))
}

pub async fn http_get_service_actor(
    Extension(service_actor): Extension<ServiceActor>,
) -> Result<impl IntoResponse, AppError> {
    Ok(FederationJson(WithContext::new_default(service_actor.clone())).into_response())
}

pub fn activitypub(service_actor: ServiceActor) -> Router {
    Router::new()
        .route("/.well-known/webfinger", get(http_get_webfinger))
        .route(
            "/ap/actor",
            get(http_get_service_actor.layer(Extension(service_actor))),
        )
}
