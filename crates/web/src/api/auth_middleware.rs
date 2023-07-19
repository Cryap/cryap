use std::sync::Arc;

use axum::{
    extract::{State, TypedHeader},
    headers::authorization::{Authorization, Bearer},
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use db::models::Session;

use crate::{api::ApiError, AppState};

pub async fn auth_middleware<B>(
    State(state): State<Arc<AppState>>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    mut request: Request<B>,
    next: Next<B>,
) -> Result<Response, ApiError> {
    let session = Session::by_token(auth.token(), &state.db_pool).await;
    if let Ok(Some(session)) = session {
        request.extensions_mut().insert(session);
        Ok(next.run(request).await)
    } else {
        Err(ApiError::new(
            "This method requires an authenticated user",
            StatusCode::UNPROCESSABLE_ENTITY,
        ))
    }
}
