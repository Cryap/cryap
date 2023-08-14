mod oauth;

use std::sync::Arc;

use axum::{
    extract::State,
    handler::Handler,
    http::StatusCode,
    middleware::from_fn_with_state,
    response::IntoResponse,
    routing::{get, post},
    Extension, Json, Router,
};
use db::models::{Application, Session};
use serde::Deserialize;
use url::Url;
use web::{errors::AppError, AppState};

use crate::{
    auth_middleware::auth_middleware, entities::Application as ApiApplication, error::ApiError,
};

#[derive(Deserialize)]
pub struct CreateBody {
    #[serde(rename = "client_name")]
    name: String,
    #[serde(rename = "redirect_uris")]
    redirect_uri: String,
    website: Option<String>,
}

// https://docs.joinmastodon.org/methods/apps/#create
pub async fn http_post_create(
    state: State<Arc<AppState>>,
    Json(body): Json<CreateBody>,
) -> Result<impl IntoResponse, AppError> {
    if Url::parse(&body.redirect_uri).is_err() {
        return Ok(ApiError::new(
            "Validation failed: Redirect URI must be an absolute URI.",
            StatusCode::UNPROCESSABLE_ENTITY,
        )
        .into_response());
    }

    let application =
        Application::create(body.name, body.website, body.redirect_uri, &state.db_pool).await?;
    Ok(Json(ApiApplication::new(application, true)).into_response())
}

// TODO: Fully implement https://docs.joinmastodon.org/methods/apps/#verify_credentials
pub async fn http_get_verify_credentials(
    state: State<Arc<AppState>>,
    Extension(session): Extension<Session>,
) -> Result<impl IntoResponse, AppError> {
    match session.application(&state.db_pool).await? {
        Some(application) => Ok(Json(ApiApplication::new(application, false)).into_response()),
        None => Ok(Json(()).into_response()), // FIXME: I don't know what Mastodon does in this case
    }
}

pub fn apps(state: &Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .merge(oauth::oauth())
        .route("/api/v1/apps", post(http_post_create))
        .route(
            "/api/v1/apps/verify_credentials",
            get(http_get_verify_credentials
                .layer(from_fn_with_state(Arc::clone(state), auth_middleware))),
        )
}
