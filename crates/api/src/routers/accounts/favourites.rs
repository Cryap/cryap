use std::sync::Arc;

use axum::{
    extract::{Query, State},
    handler::Handler,
    http::header,
    middleware::from_fn_with_state,
    response::IntoResponse,
    routing::get,
    Extension, Json, Router,
};
use db::{models::Session, pagination::PaginationQuery};
use web::{errors::AppError, AppState};

use crate::{entities::Status, routers::accounts::auth_middleware};

// https://docs.joinmastodon.org/methods/favourites/#get
pub async fn http_get_favourites(
    state: State<Arc<AppState>>,
    Extension(session): Extension<Session>,
    Query(pagination): Query<PaginationQuery>,
) -> Result<impl IntoResponse, AppError> {
    let user = session.user(&state.db_pool).await?;
    let posts = Status::build_from_vec(
        user.liked_posts(pagination.into(), &state.db_pool).await?,
        Some(&user.id),
        &state,
    )
    .await?;

    if posts.is_empty() {
        Ok(Json(posts).into_response())
    } else {
        Ok((
            [(
                header::LINK, format!(
                    "<https://{}/api/v1/favourites?max_id={}>; rel=\"next\", <https://{}/api/v1/favourites?min_id={}>; rel\"prev\"",
                    state.config.web.domain, posts.last().unwrap().id.clone(),
                    state.config.web.domain, posts.first().unwrap().id.clone()
                )
            )],
            Json(posts),
        ).into_response())
    }
}

pub fn favourites(state: &Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new().route(
        "/api/v1/favourites",
        get(http_get_favourites.layer(from_fn_with_state(Arc::clone(state), auth_middleware))),
    )
}
