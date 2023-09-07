use std::sync::Arc;

use activitypub_federation::config::{FederationConfig, FederationMiddleware};
use ap::{objects::service_actor::ServiceActor, routers::ap};
use api::routers::api;
use axum::{routing::get, Router};
use tower_http::{
    cors::{Any, CorsLayer},
    normalize_path::{NormalizePath, NormalizePathLayer},
    trace::TraceLayer,
};
use tower_layer::Layer;
use web::AppState;

use crate::frontend::ssr_handler;

pub fn app(
    federation_config: FederationConfig<Arc<AppState>>,
    service_actor: ServiceActor,
) -> NormalizePath<Router> {
    let state = Arc::clone(&*federation_config);
    let cors = CorsLayer::new()
        // allow `GET` and `POST` when accessing the resource
        .allow_methods(Any)
        .allow_headers(Any)
        // allow requests from any origin
        .allow_origin(Any);

    NormalizePathLayer::trim_trailing_slash().layer(
        Router::new()
            .merge(ap(service_actor))
            .merge(api(Arc::clone(&state)).with_state(state).layer(cors))
            .merge(crate::frontend::resources())
            .fallback_service(get(ssr_handler))
            .layer(FederationMiddleware::new(federation_config))
            .layer(TraceLayer::new_for_http()),
    )
}
