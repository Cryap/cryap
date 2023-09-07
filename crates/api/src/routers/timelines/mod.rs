mod streaming;

use std::sync::Arc;

use axum::{
    extract::State, handler::Handler, middleware::from_fn_with_state, response::IntoResponse,
    routing::get, Extension, Json, Router,
};
use db::models::Session;
use web::{errors::AppError, AppState};

use crate::auth_middleware::auth_middleware;

// https://docs.joinmastodon.org/methods/statuses/#get
pub async fn http_get_home(
    _state: State<Arc<AppState>>,
    Extension(_session): Extension<Session>,
) -> Result<impl IntoResponse, AppError> {
    Ok(Json::<Vec<()>>(vec![]))
}

pub fn timelines(state: &Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new().merge(streaming::streaming(state)).route(
        "/api/v1/timelines/home",
        get(http_get_home.layer(from_fn_with_state(Arc::clone(state), auth_middleware))),
    )
}
