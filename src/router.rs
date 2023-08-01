use std::sync::Arc;

use activitypub_federation::config::{FederationConfig, FederationMiddleware};
use ap::{objects::service_actor::ServiceActor, routers::ap};
use api::routers::api;
use axum::Router;
use tower_http::{
    trace::TraceLayer,
};
use axum::routing::get;
use web::AppState;

use crate::frontend::ssr_handler;

pub fn app(
    federation_config: FederationConfig<Arc<AppState>>,
    service_actor: ServiceActor,
) -> Router {
    let state = Arc::clone(&*federation_config);

    Router::new()
        .merge(ap(service_actor))
        .merge(api(Arc::clone(&state)).with_state(state))
        .merge(crate::frontend::resources())
        .fallback_service(get(ssr_handler))
        .layer(FederationMiddleware::new(federation_config))
        .layer(TraceLayer::new_for_http())
}
