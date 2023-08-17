use axum::{
    http::{header, StatusCode, Uri},
    response::{Html, IntoResponse, Response},
    routing::{get, Router},
};
use rust_embed::RustEmbed;

pub fn resources() -> Router {
    Router::new().route("/assets/*file", get(static_handler))
}

use std::sync::Arc;

use activitypub_federation::config::Data;
use axum::{body::Body, http::Request};
use web::AppState;

#[axum::debug_handler]
pub async fn ssr_handler(state: Data<Arc<AppState>>, req: Request<Body>) -> impl IntoResponse {
    let html = state
        .local_pool
        .spawn_pinned(|| async move { frontend::ssr::render(req.uri().to_string()).await })
        .await
        .unwrap();

    let template = Asset::get("index.html").unwrap().data;
    let template = String::from_utf8_lossy(&template.as_ref());

    let html = template.replace("<!--SSR-->", &html);

    Html(html)
}

async fn static_handler(uri: Uri) -> impl IntoResponse {
    let mut path = uri.path().trim_start_matches('/').to_string();

    if path.starts_with("assets/") {
        path = path.replace("assets/", "");
    }

    StaticFile(path)
}

#[derive(RustEmbed)]
#[folder = "$FRONTEND_DIST"]
struct Asset;

pub struct StaticFile<T>(pub T);

impl<T> IntoResponse for StaticFile<T>
where
    T: Into<String>,
{
    fn into_response(self) -> Response {
        let path = self.0.into();

        match Asset::get(path.as_str()) {
            Some(content) => {
                let mime = mime_guess::from_path(path).first_or_octet_stream();
                ([(header::CONTENT_TYPE, mime.as_ref())], content.data).into_response()
            },
            None => (StatusCode::NOT_FOUND, "404 Not Found").into_response(),
        }
    }
}
