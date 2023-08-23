#![forbid(unsafe_code)]

pub mod auth_middleware;
pub mod common;
pub mod entities;
pub mod error;
pub mod routers;

use axum::{
    response::{IntoResponse, Response},
    Json,
};
use lazy_static::lazy_static;
use serde::Serialize;
use tera::Tera;

lazy_static! {
    pub static ref TEMPLATES: Tera = match Tera::new("crates/api/templates/*") {
        Ok(template) => template,
        Err(err) => {
            log::error!("Parsing error(s): {}", err);
            ::std::process::exit(1);
        },
    };
}

#[derive(Serialize)]
pub struct EmptyJsonObject {}

impl EmptyJsonObject {
    pub fn response() -> Response {
        Self {}.into_response()
    }
}

impl IntoResponse for EmptyJsonObject {
    fn into_response(self) -> Response {
        Json(self).into_response()
    }
}
