pub mod activitypub;
pub mod posts;
pub mod users;

use axum::Router;

use crate::objects::service_actor::ServiceActor;

pub fn ap(service_actor: ServiceActor) -> Router {
    Router::new()
        .merge(activitypub::activitypub(service_actor))
        .merge(users::users())
        .merge(posts::posts())
}
