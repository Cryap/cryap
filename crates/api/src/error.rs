use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use serde_with::skip_serializing_none;

#[derive(Serialize, Debug)]
#[skip_serializing_none]
pub struct ApiError {
    pub error: String,
    #[serde(rename = "error_description")]
    pub description: Option<String>,
    #[serde(skip_serializing)]
    pub status_code: StatusCode,
}

impl ApiError {
    pub fn new(error: &str, status_code: StatusCode) -> Self {
        ApiError {
            error: String::from(error),
            description: None,
            status_code,
        }
    }

    pub fn new_with_description(error: &str, description: &str, status_code: StatusCode) -> Self {
        ApiError {
            error: String::from(error),
            description: Some(String::from(description)),
            status_code,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status_code, Json(self)).into_response()
    }
}
