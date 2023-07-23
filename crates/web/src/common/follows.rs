use std::sync::Arc;

use activitypub_federation::{
    activity_queue::send_activity, config::Data, fetch::object_id::ObjectId, traits::Actor,
};
use db::{
    models::{user::User, UserFollowRequestsInsert},
    schema::{user_follow_requests, user_follow_requests::dsl},
    types::DbId,
};
use diesel::insert_into;
use diesel_async::RunQueryDsl;
use url::Url;

use crate::{ap::activities::follow::Follow, ApUser, AppState};

pub async fn want_to_follow(
    by: &User,
    to: &User,
    data: &Data<Arc<AppState>>,
) -> Result<(), anyhow::Error> {
    let mut conn = data.db_pool.get().await?;
    let id = Url::parse(&format!(
        "{}/activities/follows/{}",
        by.ap_id,
        DbId::default()
    ))?;
    let activity = Follow {
        id: id.clone(),
        kind: Default::default(),
        actor: ObjectId::<ApUser>::from(Url::parse(&by.ap_id)?),
        object: ObjectId::<ApUser>::from(Url::parse(&to.ap_id)?),
        to: Some([ObjectId::<ApUser>::from(Url::parse(&to.ap_id)?)]),
    };

    let by = ApUser(by.clone());
    let to = ApUser(to.clone());

    let inboxes = vec![to.inbox()];
    send_activity(activity, &by, inboxes, &data).await?;

    insert_into(dsl::user_follow_requests)
        .values(vec![UserFollowRequestsInsert {
            actor_id: by.id.clone(),
            follower_id: to.id.clone(),
            ap_id: Some(id.to_string()),
        }])
        .on_conflict((
            user_follow_requests::actor_id,
            user_follow_requests::follower_id,
        ))
        .do_nothing()
        .execute(&mut conn)
        .await?;

    Ok(())
}

pub async fn unfollow(by: User, to: User, data: &Arc<AppState>) -> Result<(), anyhow::Error> {
    Ok(())
}
