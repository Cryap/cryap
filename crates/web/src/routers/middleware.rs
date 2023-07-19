use anyhow::anyhow;
use axum::body::Bytes;
use axum::middleware::Next;
use axum::response::IntoResponse;
use hyper::{Body, Request};
use serde_json::Value;

use crate::errors::AppError;

async fn buffer_and_print<B>(body: B) -> Result<Bytes, AppError>
where
    B: axum::body::HttpBody<Data = Bytes>,
    B::Error: std::fmt::Display,
{
    let bytes = match hyper::body::to_bytes(body).await {
        Ok(bytes) => bytes,
        Err(err) => {
            return Err(AppError::from(anyhow!("failed to read body: {}", err)));
        }
    };

    if let Ok(body) = std::str::from_utf8(&bytes) {
        let json: Value = serde_json::from_str(body)?;
        log::debug!("Inbox: {:#?}", json);
    }

    Ok(bytes)
}

pub async fn print_inbox(
    req: Request<Body>,
    next: Next<Body>,
) -> Result<impl IntoResponse, AppError> {
    let (parts, body) = req.into_parts();
    let bytes = buffer_and_print(body).await?;
    let req = Request::from_parts(parts, Body::from(bytes));

    let res = next.run(req).await;

    Ok(res)
}
