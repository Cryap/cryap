use db::{
    models::{Notification, Post, User},
    types::DbNotificationType,
};
use diesel_async::{pooled_connection::deadpool::Pool, AsyncPgConnection};

pub async fn process_follow(
    by: &User,
    to: &User,
    do_opposite: bool,
    db_pool: &Pool<AsyncPgConnection>,
) -> anyhow::Result<()> {
    if !to.local {
        return Ok(());
    }

    if do_opposite {
        println!("{} {}", &by.id, &to.id);
        Notification::delete(by, to, None, DbNotificationType::Follow, db_pool).await?;
    } else {
        Notification::create(by, to, None, DbNotificationType::Follow, db_pool).await?;
    }

    Ok(())
}

pub async fn process_follow_request(
    by: &User,
    to: &User,
    do_opposite: bool,
    db_pool: &Pool<AsyncPgConnection>,
) -> anyhow::Result<()> {
    if !to.local {
        return Ok(());
    }

    if do_opposite {
        Notification::delete(by, to, None, DbNotificationType::FollowRequest, db_pool).await?;
    } else {
        Notification::create(by, to, None, DbNotificationType::FollowRequest, db_pool).await?;
    }

    Ok(())
}

pub async fn process_post(post: &Post, db_pool: &Pool<AsyncPgConnection>) -> anyhow::Result<()> {
    for user in post.local_mentioned_users(db_pool).await? {
        Notification::create_by_ids(
            post.author.clone(),
            user.id.clone(),
            Some(post.id.clone()),
            DbNotificationType::Mention,
            db_pool,
        )
        .await?;
    }

    Ok(())
}

pub async fn process_like(
    post: &Post,
    by: &User,
    do_opposite: bool,
    db_pool: &Pool<AsyncPgConnection>,
) -> anyhow::Result<()> {
    let author = post.author(db_pool).await?;
    if !author.local {
        return Ok(());
    }

    if do_opposite {
        Notification::delete(
            by,
            &author,
            Some(post),
            DbNotificationType::Favourite,
            db_pool,
        )
        .await?;
    } else {
        Notification::create(
            by,
            &author,
            Some(post),
            DbNotificationType::Favourite,
            db_pool,
        )
        .await?;
    }

    Ok(())
}

pub async fn process_boost(
    post: &Post,
    by: &User,
    do_opposite: bool,
    db_pool: &Pool<AsyncPgConnection>,
) -> anyhow::Result<()> {
    let author = post.author(db_pool).await?;
    if !author.local {
        return Ok(());
    }

    if do_opposite {
        Notification::delete(by, &author, Some(post), DbNotificationType::Reblog, db_pool).await?;
    } else {
        Notification::create(by, &author, Some(post), DbNotificationType::Reblog, db_pool).await?;
    }

    Ok(())
}
