pub mod activitypub;
pub mod users;

use axum::Router;
use web::AppState;

use crate::objects::service_actor::ServiceActor;

pub fn ap(service_actor: ServiceActor) -> Router {
    Router::new()
        .merge(activitypub::activitypub(service_actor))
        .merge(users::users())
}
