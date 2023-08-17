use std::sync::Arc;

use axum::{
    body::Body,
    extract::{Query, State},
    http::{Request, StatusCode},
    response::{Html, IntoResponse, Redirect},
    routing::{get, post},
    Form, Json, Router,
};
use axum_extra::extract::cookie::CookieJar;
use db::models::{Application, RedirectCode, Session};
use serde::{Deserialize, Serialize};
use tera::Context;
use url::Url;
use web::{errors::AppError, AppState};

use crate::{entities::Token, error::ApiError, TEMPLATES};

#[derive(Deserialize)]
pub struct AuthorizeQuery {
    response_type: Option<String>,
    client_id: String,
    #[serde(rename = "redirect_uri")]
    redirect_url: String,
}

// TODO: Fully implement https://docs.joinmastodon.org/methods/oauth/#authorize
pub async fn http_get_oauth_authorize(
    state: State<Arc<AppState>>,
    jar: CookieJar,
    Query(query): Query<AuthorizeQuery>,
    request: Request<Body>,
) -> Result<impl IntoResponse, AppError> {
    let session = match jar.get("token") {
        Some(token) => match Session::by_token(token.value(), &state.db_pool).await? {
            Some(session) => session,
            None => {
                return Ok(Redirect::to(&format!(
                    "/auth/sign_in?redirect=/oauth/authorize?{}",
                    request.uri().query().unwrap_or("")
                ))
                .into_response());
            },
        },
        None => {
            return Ok(Redirect::to(&format!(
                "/auth/sign_in?redirect=/oauth/authorize?{}",
                request.uri().query().unwrap_or("")
            ))
            .into_response());
        },
    };

    let application = match Application::by_client_id(&query.client_id, &state.db_pool).await? {
        Some(application)
            if query.response_type.unwrap_or(String::from("code")) == "code"
                && application.redirect_url == query.redirect_url =>
        {
            application
        },
        _ => {
            return Ok(StatusCode::BAD_REQUEST.into_response());
        },
    };

    let user = session.user(&state.db_pool).await?;
    let mut context = Context::new();
    context.insert("title", "Cryap"); // TODO: Add ability to specify it in config
    context.insert("username", &user.display_name.unwrap_or(user.name));
    context.insert("application_name", &application.name);
    context.insert("client_id", &query.client_id);
    context.insert("redirect_url", &query.redirect_url);
    Ok(Html(TEMPLATES.render("authorize.html", &context)?).into_response())
}

#[derive(Deserialize)]
pub struct AuthorizeBody {
    client_id: String,
    redirect_url: String,
}

pub async fn http_post_oauth_authorize(
    state: State<Arc<AppState>>,
    jar: CookieJar,
    Form(body): Form<AuthorizeBody>,
) -> Result<impl IntoResponse, AppError> {
    let session = match jar.get("token") {
        Some(token) => match Session::by_token(token.value(), &state.db_pool).await? {
            Some(session) => session,
            None => {
                return Ok(StatusCode::UNAUTHORIZED.into_response());
            },
        },
        None => {
            return Ok(StatusCode::UNAUTHORIZED.into_response());
        },
    };

    let application = match Application::by_client_id(&body.client_id, &state.db_pool).await? {
        Some(application) if application.redirect_url == body.redirect_url => application,
        _ => {
            return Ok(StatusCode::BAD_REQUEST.into_response());
        },
    };

    let redirect_code = RedirectCode::create(
        application.client_id,
        session.user(&state.db_pool).await?.id,
        &mut state.redis.clone(),
    )
    .await?;

    if body.redirect_url == "urn:ietf:wg:oauth:2.0:oob" {
        Ok(Redirect::to(&format!(
            "/oauth/authorize/native?code={}",
            &redirect_code.code
        ))
        .into_response())
    } else {
        let mut url = Url::parse(&body.redirect_url)?;
        url.query_pairs_mut()
            .append_pair("code", &redirect_code.code);
        Ok(Redirect::to(&url.to_string()).into_response())
    }
}

#[derive(Deserialize)]
pub struct TokenBody {
    code: String,
    client_id: String,
    client_secret: String,
    #[serde(rename = "redirect_uri")]
    redirect_url: String,
}

// TODO: Fully implement https://docs.joinmastodon.org/methods/oauth/#token
pub async fn http_post_oauth_token(
    state: State<Arc<AppState>>,
    Form(body): Form<TokenBody>,
) -> Result<impl IntoResponse, AppError> {
    let application = match Application::by_client_id(&body.client_id, &state.db_pool).await? {
        Some(application) if application.client_secret == body.client_secret => application,
        _ => {
            return Ok(ApiError::new_with_description("invalid_client", "Client authentication failed due to unknown client, no client authentication included, or unsupported authentication method.", StatusCode::UNAUTHORIZED).into_response());
        },
    };

    let mut redis = state.redis.clone();
    let redirect_code = match RedirectCode::by_code(&body.code, &mut redis).await? {
        Some(redirect_code) if application.redirect_url == body.redirect_url => redirect_code,
        _ => {
            return Ok(ApiError::new_with_description("invalid_grant", "The provided authorization grant is invalid, expired, revoked, does not match the redirection URI used in the authorization request, or was issued to another client.", StatusCode::UNAUTHORIZED).into_response());
        },
    };

    let session = Session::create(
        redirect_code.user(&state.db_pool).await?.id,
        Some(application.id),
        &state.db_pool,
    )
    .await?;
    redirect_code.delete(&mut redis).await?;
    Ok(Json(Token::new(session)).into_response())
}

#[derive(Deserialize)]
pub struct RevokeBody {
    token: String,
    client_id: String,
    client_secret: String,
}

#[derive(Serialize)]
struct EmptyJsonObject {}

// https://docs.joinmastodon.org/methods/oauth/#revoke
pub async fn http_post_oauth_revoke(
    state: State<Arc<AppState>>,
    Json(body): Json<RevokeBody>,
) -> Result<impl IntoResponse, AppError> {
    let session = match Session::by_token(&body.token, &state.db_pool).await? {
        Some(session) => session,
        None => {
            return Ok(Json(EmptyJsonObject {}).into_response());
        },
    };

    match Application::by_client_id(&body.client_id, &state.db_pool).await? {
        Some(application)
            if application.client_secret == body.client_secret
                && session.application(&state.db_pool).await?.is_some_and(
                    |session_application| session_application.id == application.id,
                ) =>
        {
            session.delete(&state.db_pool).await?;
            Ok(Json(EmptyJsonObject {}).into_response())
        },
        _ => Ok(ApiError::new_with_description(
            "unauthorized_client",
            "You are not authorized to revoke this token",
            StatusCode::FORBIDDEN,
        )
        .into_response()),
    }
}

pub fn oauth() -> Router<Arc<AppState>> {
    Router::new()
        .route("/oauth/authorize", get(http_get_oauth_authorize))
        .route("/oauth/authorize", post(http_post_oauth_authorize))
        .route("/oauth/token", post(http_post_oauth_token))
        .route("/oauth/revoke", post(http_post_oauth_revoke))
}
