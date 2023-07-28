use std::sync::Arc;

use activitypub_federation::config::{FederationConfig, FederationMiddleware};
use ap::{objects::service_actor::ServiceActor, routers::ap};
use api::routers::api;
use axum::Router;
use tower_http::{
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};
use web::AppState;

pub fn app(
    federation_config: FederationConfig<Arc<AppState>>,
    service_actor: ServiceActor,
) -> Router {
    let serve_dir = ServeDir::new(format!("{}/../frontend/dist", env!("CARGO_MANIFEST_DIR")))
        .not_found_service(ServeFile::new(format!(
            "{}/../frontend/dist/index.html",
            env!("CARGO_MANIFEST_DIR")
        )));
    let state = Arc::clone(&*federation_config);

    Router::new()
        .merge(ap(service_actor))
        .merge(api(Arc::clone(&state)).with_state(state))
        .layer(FederationMiddleware::new(federation_config))
        .layer(TraceLayer::new_for_http())
        .fallback_service(serve_dir)
}
