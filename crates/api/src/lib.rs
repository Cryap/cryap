#![forbid(unsafe_code)]

pub mod auth_middleware;
pub mod entities;
pub mod error;
pub mod routers;

use lazy_static::lazy_static;
use tera::Tera;

lazy_static! {
    pub static ref TEMPLATES: Tera = match Tera::new("crates/api/templates/*") {
        Ok(template) => template,
        Err(err) => {
            log::error!("Parsing error(s): {}", err);
            ::std::process::exit(1);
        }
    };
}
