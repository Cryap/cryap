use std::sync::Arc;

use axum::{extract::State, http::StatusCode, response::IntoResponse, Extension, Json};
use db::models::{Application, Session};
use serde::Deserialize;
use url::Url;
use web::{errors::AppError, AppState};

use crate::{entities::Application as ApiApplication, error::ApiError};

// TODO: Make private after `nest` fix
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
