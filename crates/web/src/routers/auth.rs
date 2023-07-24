use std::sync::Arc;

use argon2::{Argon2, PasswordHash, PasswordVerifier};
use axum::{extract::State, response::IntoResponse, Json};
use db::models::{Session, User};
use serde::Deserialize;

use crate::{api::entities::Token, errors::AppError, AppState};

#[derive(Deserialize)]
pub struct PostLoginBody {
    name: String,
    password: String,
}

// TODO: OAuth2
pub async fn http_post_login(
    state: State<Arc<AppState>>,
    Json(body): Json<PostLoginBody>,
) -> Result<impl IntoResponse, AppError> {
    let user = User::by_name(&body.name, &state.db_pool).await?;

    if let Some(user) = user {
        let hash = user.password_encrypted.unwrap();
        let parsed_hash = PasswordHash::new(&hash).unwrap();
        if Argon2::default()
            .verify_password(body.password.as_bytes(), &parsed_hash)
            .is_err()
        {
            return Ok(String::from("invalid password!!!").into_response());
        }

        let session = Session::new(user.id, &state.db_pool).await?;
        Ok(Json(Token::new(session)).into_response())
    } else {
        Ok(String::from("not found!").into_response())
    }
}
