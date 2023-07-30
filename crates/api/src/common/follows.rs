use std::sync::Arc;

use activitypub_federation::{
    activity_queue::send_activity, config::Data, fetch::object_id::ObjectId, traits::Actor,
};
use ap::{
    activities::{follow::Follow, reject::follow::RejectFollow, undo::follow::UndoFollow},
    objects::user::ApUser,
};
use db::{
    models::{user::User, UserFollowRequestsInsert},
    schema::{user_follow_requests, user_followers},
    types::DbId,
};
use diesel::{delete, insert_into, result::Error::NotFound, ExpressionMethods, QueryDsl};
use diesel_async::RunQueryDsl;
use url::Url;
use web::AppState;

pub async fn want_to_follow(
    by: &User,
    to: &User,
    data: &Data<Arc<AppState>>,
) -> anyhow::Result<()> {
    let mut conn = data.db_pool.get().await?;
    let id = Url::parse(&format!(
        "{}/activities/follows/{}",
        by.ap_id,
        DbId::default().to_string()
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

    let inboxes = vec![to.shared_inbox_or_inbox()];
    send_activity(activity, &by, inboxes, &data).await?;

    insert_into(user_follow_requests::dsl::user_follow_requests)
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

pub async fn unfollow(by: &User, to: &User, data: &Data<Arc<AppState>>) -> anyhow::Result<()> {
    let mut conn = data.db_pool.get().await?;
    let undo_id = Url::parse(&format!(
        "{}/activities/undo/follows/{}",
        by.ap_id,
        DbId::default()
    ))?;
    let follow_id = user_followers::table
        .select(user_followers::ap_id)
        .filter(user_followers::actor_id.eq(by.id.clone()))
        .filter(user_followers::follower_id.eq(to.id.clone()))
        .first::<Option<String>>(&mut conn)
        .await;
    let follow_id = Url::parse(
        &match follow_id {
            Ok(follow_id) => follow_id,
            Err(NotFound) => {
                user_follow_requests::table
                    .select(user_follow_requests::ap_id)
                    .filter(user_follow_requests::actor_id.eq(by.id.clone()))
                    .filter(user_follow_requests::follower_id.eq(to.id.clone()))
                    .first::<Option<String>>(&mut conn)
                    .await?
            }
            Err(err) => return Err(err.into()),
        }
        .unwrap_or_else(|| String::new()),
    )?;

    let activity = UndoFollow {
        actor: ObjectId::<ApUser>::from(Url::parse(&by.ap_id)?),
        to: Some([ObjectId::<ApUser>::from(Url::parse(&to.ap_id)?)]),
        object: Follow {
            id: follow_id,
            kind: Default::default(),
            actor: ObjectId::<ApUser>::from(Url::parse(&by.ap_id)?),
            object: ObjectId::<ApUser>::from(Url::parse(&to.ap_id)?),
            to: Some([ObjectId::<ApUser>::from(Url::parse(&to.ap_id)?)]),
        },
        kind: Default::default(),
        id: undo_id,
    };

    let by = ApUser(by.clone());
    let to = ApUser(to.clone());

    let inboxes = vec![to.shared_inbox_or_inbox()];
    send_activity(activity, &by, inboxes, &data).await?;

    let _ = delete(
        user_follow_requests::dsl::user_follow_requests
            .filter(user_follow_requests::actor_id.eq(by.id.clone()))
            .filter(user_follow_requests::follower_id.eq(to.id.clone())),
    )
    .execute(&mut conn)
    .await;

    let _ = delete(
        user_followers::dsl::user_followers
            .filter(user_followers::actor_id.eq(by.id.clone()))
            .filter(user_followers::follower_id.eq(to.id.clone())),
    )
    .execute(&mut conn)
    .await;

    Ok(())
}

pub async fn remove_from_followers(
    by: &User,
    to: &User,
    data: &Data<Arc<AppState>>,
) -> anyhow::Result<()> {
    let mut conn = data.db_pool.get().await?;
    let reject_id = Url::parse(&format!(
        "{}/activities/reject/follows/{}",
        by.ap_id,
        DbId::default()
    ))?;
    let follow_id = user_followers::table
        .select(user_followers::ap_id)
        .filter(user_followers::actor_id.eq(to.id.clone()))
        .filter(user_followers::follower_id.eq(by.id.clone()))
        .first::<Option<String>>(&mut conn)
        .await;
    let follow_id = Url::parse(
        &match follow_id {
            Ok(follow_id) => follow_id,
            Err(NotFound) => {
                user_follow_requests::table
                    .select(user_follow_requests::ap_id)
                    .filter(user_follow_requests::actor_id.eq(to.id.clone()))
                    .filter(user_follow_requests::follower_id.eq(by.id.clone()))
                    .first::<Option<String>>(&mut conn)
                    .await?
            }
            Err(err) => return Err(err.into()),
        }
        .unwrap_or_else(|| String::new()),
    )?;

    let activity = RejectFollow {
        actor: ObjectId::<ApUser>::from(Url::parse(&by.ap_id)?),
        to: Some([ObjectId::<ApUser>::from(Url::parse(&to.ap_id)?)]),
        object: Follow {
            id: follow_id,
            kind: Default::default(),
            actor: ObjectId::<ApUser>::from(Url::parse(&to.ap_id)?),
            object: ObjectId::<ApUser>::from(Url::parse(&by.ap_id)?),
            to: Some([ObjectId::<ApUser>::from(Url::parse(&by.ap_id)?)]),
        },
        kind: Default::default(),
        id: reject_id,
    };

    let by = ApUser(by.clone());
    let to = ApUser(to.clone());

    let inboxes = vec![to.shared_inbox_or_inbox()];
    send_activity(activity, &by, inboxes, &data).await?;

    let _ = delete(
        user_follow_requests::dsl::user_follow_requests
            .filter(user_follow_requests::actor_id.eq(to.id.clone()))
            .filter(user_follow_requests::follower_id.eq(by.id.clone())),
    )
    .execute(&mut conn)
    .await;

    let _ = delete(
        user_followers::dsl::user_followers
            .filter(user_followers::actor_id.eq(to.id.clone()))
            .filter(user_followers::follower_id.eq(by.id.clone())),
    )
    .execute(&mut conn)
    .await;

    Ok(())
}
