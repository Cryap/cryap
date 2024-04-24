use std::sync::Arc;

use activitypub_federation::config::Data;
use ap::{
    activities::{
        accept::follow::AcceptFollow, follow::Follow, reject::follow::RejectFollow,
        undo::follow::UndoFollow,
    },
    common::notifications,
    objects::user::ApUser,
};
use db::models::{user::User, user_follow_request::UserFollowRequest, user_follower::UserFollower};
use url::Url;
use web::AppState;

pub async fn want_to_follow(
    by: &User,
    to: &User,
    data: &Data<Arc<AppState>>,
) -> anyhow::Result<()> {
    if to.local {
        if to.manually_approves_followers {
            UserFollowRequest::create(&by, &to, None, &data.db_pool).await?;
            notifications::process_follow_request(&by, &to, false, &data.db_pool).await?;
        } else {
            UserFollower::create(&by, &to, None, &data.db_pool).await?;
            notifications::process_follow(&by, &to, false, &data.db_pool).await?;
        }
    } else {
        let id = Follow::send(&ApUser(by.clone()), &ApUser(to.clone()), data)
            .await?
            .to_string();
        UserFollowRequest::create(&by, &to, Some(id), &data.db_pool).await?;
    }

    Ok(())
}

pub async fn unfollow(by: &User, to: &User, data: &Data<Arc<AppState>>) -> anyhow::Result<()> {
    if !to.local {
        let follow_id = by.follow_id(&to, &data.db_pool).await?;
        if let Some(follow_id) = follow_id {
            UndoFollow::send(
                Url::parse(&follow_id)?,
                &ApUser(by.clone()),
                &ApUser(to.clone()),
                data,
            )
            .await?;
        } else {
            return Ok(());
        }
    }

    if UserFollowRequest::delete(&by, &to, None, &data.db_pool).await? {
        notifications::process_follow_request(&by, &to, true, &data.db_pool).await?;
    } else {
        UserFollower::delete(&by, &to, None, &data.db_pool).await?;
        notifications::process_follow(&by, &to, true, &data.db_pool).await?;
    }

    Ok(())
}

pub async fn accept_follow_request(
    by: &User,
    to: &User,
    request_id: String,
    data: &Data<Arc<AppState>>,
) -> anyhow::Result<()> {
    if !by.local {
        AcceptFollow::send(
            Url::parse(&request_id)?,
            &ApUser(to.clone()),
            &ApUser(by.clone()),
            data,
        )
        .await?;
    }

    UserFollowRequest::delete(&by, &to, None, &data.db_pool).await?;
    UserFollower::create(&by, &to, Some(request_id), &data.db_pool).await?;

    Ok(())
}

pub async fn remove_from_followers(
    by: &User,
    to: &User,
    request_id: Option<String>,
    data: &Data<Arc<AppState>>,
) -> anyhow::Result<()> {
    if !to.local {
        let follow_id = if let Some(request_id) = request_id {
            request_id
        } else if let Some(follow_id) = to.follow_id(&by, &data.db_pool).await? {
            follow_id
        } else {
            return Ok(());
        };

        RejectFollow::send(
            Url::parse(&follow_id)?,
            &ApUser(by.clone()),
            &ApUser(to.clone()),
            data,
        )
        .await?;
    }

    if !UserFollowRequest::delete(&to, &by, None, &data.db_pool).await? {
        UserFollower::delete(&to, &by, None, &data.db_pool).await?;
    }

    Ok(())
}
