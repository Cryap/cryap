use std::sync::Arc;

use activitypub_federation::{
    config::Data,
    fetch::webfinger::{extract_webfinger_name, Webfinger, WebfingerLink},
};
use axum::{extract::Query, response::IntoResponse, routing::get, Json, Router};
use db::{models::User, schema};
use diesel::{ExpressionMethods, QueryDsl};
use diesel_async::RunQueryDsl;
use serde::Deserialize;
use url::Url;
use web::{errors::AppError, AppState};

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
        .filter(schema::users::name.eq(name))
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

pub fn webfinger() -> Router {
    Router::new().route("/.well-known/webfinger", get(http_get_webfinger))
}
