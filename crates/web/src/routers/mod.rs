mod api;
mod auth;
mod middleware;
mod users;

use std::sync::Arc;

use activitypub_federation::config::Data;
use activitypub_federation::config::{FederationConfig, FederationMiddleware};
use activitypub_federation::fetch::webfinger::extract_webfinger_name;
use activitypub_federation::fetch::webfinger::{Webfinger, WebfingerLink};
use axum::extract::Query;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{handler::Handler, middleware::from_fn_with_state, Json, Router};
use db::models::User;
use db::schema;
use diesel::ExpressionMethods;
use diesel::QueryDsl;
use diesel_async::RunQueryDsl;
use serde::Deserialize;
use tower_http::services::{ServeDir, ServeFile};
use url::Url;

use crate::api::auth_middleware;
use crate::errors::AppError;
use crate::AppState;

#[derive(Deserialize)]
struct WebfingerQuery {
    resource: String,
}

async fn http_get_webfinger(
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

pub fn app(federation_config: FederationConfig<Arc<AppState>>) -> Router {
    let serve_dir = ServeDir::new(format!("{}/../frontend/dist", env!("CARGO_MANIFEST_DIR")))
        .not_found_service(ServeFile::new(format!(
            "{}/../frontend/dist/index.html",
            env!("CARGO_MANIFEST_DIR")
        )));
    let state = Arc::clone(&*federation_config);

    Router::new()
        .route("/.well-known/webfinger", get(http_get_webfinger))
        .route(
            "/u/:name/ap/inbox",
            post(
                users::http_post_user_inbox
                    .layer(axum::middleware::from_fn(middleware::print_inbox)),
            ),
        )
        .route("/u/:name", get(users::http_get_user))
        .route("/api/login", post(auth::http_post_login))
        .route(
            "/api/v1/accounts/verify_credentials",
            get(api::accounts::http_get_verify_credentials
                .layer(from_fn_with_state(Arc::clone(&state), auth_middleware))),
        )
        .route(
            "/api/v1/accounts/lookup",
            get(api::accounts::http_get_lookup),
        )
        .route("/api/v1/accounts/:id", get(api::accounts::http_get_get))
        .route(
            "/api/v1/accounts/:id/follow",
            post(
                api::accounts::http_post_follow
                    .layer(from_fn_with_state(Arc::clone(&state), auth_middleware)),
            ),
        )
        .route(
            "/api/v1/accounts/:id/unfollow",
            post(
                api::accounts::http_post_unfollow
                    .layer(from_fn_with_state(Arc::clone(&state), auth_middleware)),
            ),
        )
        //        .nest("/u", users())
        .with_state(state)
        .layer(FederationMiddleware::new(federation_config))
        .fallback_service(serve_dir)
}
