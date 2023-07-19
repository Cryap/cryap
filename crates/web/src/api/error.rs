use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

#[derive(Serialize, Debug)]
pub struct ApiError {
    pub error: String,
    #[serde(skip_serializing)]
    pub status_code: StatusCode,
}

impl ApiError {
    pub fn new(error: &str, status_code: StatusCode) -> Self {
        ApiError {
            error: String::from(error),
            status_code,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status_code, Json(self)).into_response()
    }
}
