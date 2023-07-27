use std::sync::Arc;

use activitypub_federation::config::{FederationConfig, FederationMiddleware};
use ap::{
    objects::service_actor::ServiceActor,
    routers::{activitypub, users},
};
use api::{
    auth_middleware::auth_middleware,
    routers::{accounts, apps, auth},
};
use axum::{
    handler::Handler,
    middleware::from_fn_with_state,
    routing::{get, post},
    Extension, Router,
};
use tower_http::{
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};
use web::AppState;

use crate::middleware;

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
        .route(
            "/.well-known/webfinger",
            get(activitypub::http_get_webfinger),
        )
        .route(
            "/u/:name/ap/inbox",
            post(
                users::http_post_user_inbox
                    .layer(axum::middleware::from_fn(middleware::print_inbox)),
            ),
        )
        .route("/u/:name", get(users::http_get_user))
        .route(
            "/ap/actor",
            get(activitypub::http_get_service_actor.layer(Extension(service_actor))),
        )
        .route("/auth/sign_in", get(auth::http_get_sign_in))
        .route("/auth/sign_in", post(auth::http_post_sign_in))
        .route("/oauth/authorize", get(auth::http_get_oauth_authorize))
        .route("/oauth/authorize", post(auth::http_post_oauth_authorize))
        .route("/oauth/token", post(auth::http_post_oauth_token))
        .route("/oauth/revoke", post(auth::http_post_oauth_revoke))
        .route(
            "/oauth/authorize/native",
            get(auth::http_get_oauth_authorize_native),
        )
        .route(
            "/api/v1/accounts/verify_credentials",
            get(accounts::http_get_verify_credentials
                .layer(from_fn_with_state(Arc::clone(&state), auth_middleware))),
        )
        .route("/api/v1/accounts/lookup", get(accounts::http_get_lookup))
        .route("/api/v1/accounts/:id", get(accounts::http_get_get))
        .route(
            "/api/v1/accounts/:id/follow",
            post(
                accounts::http_post_follow
                    .layer(from_fn_with_state(Arc::clone(&state), auth_middleware)),
            ),
        )
        .route(
            "/api/v1/accounts/:id/unfollow",
            post(
                accounts::http_post_unfollow
                    .layer(from_fn_with_state(Arc::clone(&state), auth_middleware)),
            ),
        )
        .route(
            "/api/v1/accounts/:id/remove_from_followers",
            post(
                accounts::http_post_remove_from_followers
                    .layer(from_fn_with_state(Arc::clone(&state), auth_middleware)),
            ),
        )
        .route(
            "/api/v1/accounts/relationships",
            get(accounts::http_get_relationships
                .layer(from_fn_with_state(Arc::clone(&state), auth_middleware))),
        )
        .route("/api/v1/apps", post(apps::http_post_create))
        .route(
            "/api/v1/apps/verify_credentials",
            get(apps::http_get_verify_credentials
                .layer(from_fn_with_state(Arc::clone(&state), auth_middleware))),
        )
        //        .nest("/u", users())
        .with_state(state)
        .layer(FederationMiddleware::new(federation_config))
        .layer(TraceLayer::new_for_http())
        .fallback_service(serve_dir)
}
