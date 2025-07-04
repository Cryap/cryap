use std::sync::Arc;

use argon2::{Argon2, PasswordHash, PasswordVerifier};
use axum::{
    extract::{Query, State},
    response::{Html, IntoResponse, Redirect},
    routing::{get, post},
    Form, Router,
};
use axum_extra::extract::cookie::{Cookie, CookieJar};
use db::models::{Session, User};
use serde::Deserialize;
use tera::Context;
use web::{errors::AppError, AppState};

use crate::TEMPLATES;

#[derive(Deserialize)]
pub struct SignInQuery {
    redirect_url: Option<String>,
}

pub async fn http_get_sign_in(
    state: State<Arc<AppState>>,
    jar: CookieJar,
    Query(query): Query<SignInQuery>,
) -> Result<impl IntoResponse, AppError> {
    if let Some(token) = jar.get("token") {
        if Session::by_token(token.value(), &state.db_pool)
            .await?
            .is_some()
        {
            return Ok(
                Redirect::to(&query.redirect_url.unwrap_or(String::from("/"))).into_response(),
            );
        }
    }

    let mut context = Context::new();
    context.insert("title", &state.config.instance.title);
    context.insert("redirect_url", &query.redirect_url);
    Ok(Html(TEMPLATES.render("sign_in.html", &context)?).into_response())
}

#[derive(Deserialize)]
pub struct SignInBody {
    username: String,
    password: String,
    redirect_url: Option<String>,
}

pub async fn http_post_sign_in(
    state: State<Arc<AppState>>,
    jar: CookieJar,
    Form(body): Form<SignInBody>,
) -> Result<impl IntoResponse, AppError> {
    let redirect_url = body
        .redirect_url
        .and_then(|url| if url.is_empty() { None } else { Some(url) })
        .unwrap_or(String::from("/"));

    if let Some(token) = jar.get("token") {
        if Session::by_token(token.value(), &state.db_pool)
            .await?
            .is_some()
        {
            return Ok(Redirect::to(&redirect_url).into_response());
        }
    }

    let user = User::local_by_name(&body.username, &state.db_pool).await?;
    let mut context = Context::new();

    if let Some(user) = user {
        let hash = user.password_encrypted.unwrap();
        let parsed_hash = PasswordHash::new(&hash).unwrap();
        if Argon2::default()
            .verify_password(body.password.as_bytes(), &parsed_hash)
            .is_ok()
        {
            let session = Session::create(user.id, None, &state.db_pool).await?;

            return Ok((
                jar.add(
                    Cookie::build("token", session.token)
                        .path("/")
                        .secure(true)
                        .finish(),
                ),
                Redirect::to(&redirect_url),
            )
                .into_response());
        } else {
            context.insert("invalid", &true);
        }
    } else {
        context.insert("invalid", &true);
    }

    context.insert("title", &state.config.instance.title);
    context.insert("redirect_url", &redirect_url);
    Ok(Html(TEMPLATES.render("sign_in.html", &context)?).into_response())
}

#[derive(Deserialize)]
pub struct AuthorizeNativeQuery {
    code: Option<String>,
}

pub async fn http_get_oauth_authorize_native(
    state: State<Arc<AppState>>,
    jar: CookieJar,
    Query(query): Query<AuthorizeNativeQuery>,
) -> Result<impl IntoResponse, AppError> {
    let session = match jar.get("token") {
        Some(token) => match Session::by_token(token.value(), &state.db_pool).await? {
            Some(session) => session,
            None => {
                return Ok(Redirect::to("/auth/sign_in").into_response());
            },
        },
        None => {
            return Ok(Redirect::to("/auth/sign_in").into_response());
        },
    };

    let user = session.user(&state.db_pool).await?;
    let mut context = Context::new();
    context.insert("title", &state.config.instance.title);
    context.insert("username", &user.display_name.unwrap_or(user.name));
    context.insert("code", &query.code.unwrap_or_default());
    Ok(Html(TEMPLATES.render("code_display.html", &context)?).into_response())
}

pub fn auth() -> Router<Arc<AppState>> {
    Router::new()
        .route("/auth/sign_in", get(http_get_sign_in))
        .route("/auth/sign_in", post(http_post_sign_in))
        .route(
            "/oauth/authorize/native",
            get(http_get_oauth_authorize_native),
        )
}
