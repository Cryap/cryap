pub mod activitypub;
pub mod nodeinfo;
pub mod posts;
pub mod users;
pub mod webfinger;

use axum::Router;

use crate::objects::service_actor::ServiceActor;

pub fn ap(service_actor: ServiceActor) -> Router {
    Router::new()
        .merge(activitypub::activitypub(service_actor))
        .merge(nodeinfo::nodeinfo())
        .merge(users::users())
        .merge(posts::posts())
        .merge(webfinger::webfinger())
}
