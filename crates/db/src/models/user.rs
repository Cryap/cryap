use chrono::{DateTime, Utc};
use diesel::{dsl::sql, prelude::*, query_dsl::QueryDsl, result::Error::NotFound, sql_types::Bool};
use diesel_async::{pooled_connection::deadpool::Pool, AsyncPgConnection, RunQueryDsl};

use crate::{
    common::timelines::{self, TimelineEntry},
    models::{user_follow_requests::UserFollowRequest, Post},
    paginate,
    pagination::Pagination,
    schema::{bookmarks, post_like, posts, user_follow_requests, user_followers, users},
    types::DbId,
    utils::coalesce,
};

#[derive(
    Queryable, Identifiable, Selectable, Insertable, AsChangeset, Debug, PartialEq, Clone, Eq,
)]
#[diesel(table_name = users)]
pub struct User {
    pub id: DbId,
    pub ap_id: String,
    pub local: bool,
    pub inbox_uri: String,
    pub shared_inbox_uri: Option<String>,
    pub outbox_uri: String,
    pub followers_uri: String,
    pub name: String,
    pub instance: String,
    pub display_name: Option<String>,
    pub bio: Option<String>,
    pub password_encrypted: Option<String>,
    pub admin: bool,
    pub public_key: String,
    pub private_key: Option<String>,
    pub published: DateTime<Utc>,
    pub updated: Option<DateTime<Utc>>,
    pub manually_approves_followers: bool,
    pub is_cat: bool,
    pub bot: bool,
}

#[derive(AsChangeset, Clone)]
#[diesel(table_name = users)]
// When you want to null out a column, you have to send Some(None)), since sending None means you just don't want to update that column
pub struct UserUpdate {
    pub name: Option<String>,
    pub display_name: Option<Option<String>>,
    pub bio: Option<Option<String>>,
    pub password_encrypted: Option<Option<String>>,
    pub admin: Option<bool>,
    pub updated: Option<Option<DateTime<Utc>>>,
    pub manually_approves_followers: Option<bool>,
    pub is_cat: Option<bool>,
    pub bot: Option<bool>,
}

impl User {
    pub async fn by_id(
        id: &DbId,
        db_pool: &Pool<AsyncPgConnection>,
    ) -> anyhow::Result<Option<Self>> {
        let user = users::table
            .filter(users::id.eq(id))
            .first::<Self>(&mut db_pool.get().await?)
            .await;
        match user {
            Ok(user) => Ok(Some(user)),
            Err(NotFound) => Ok(None),
            Err(err) => Err(err.into()),
        }
    }

    pub async fn local_by_name(
        name: &str,
        db_pool: &Pool<AsyncPgConnection>,
    ) -> anyhow::Result<Option<Self>> {
        let user = users::table
            .filter(users::local.eq(true))
            .filter(users::name.eq(name.to_string()))
            .first::<Self>(&mut db_pool.get().await?)
            .await;
        match user {
            Ok(user) => Ok(Some(user)),
            Err(NotFound) => Ok(None),
            Err(err) => Err(err.into()),
        }
    }

    pub async fn by_acct(
        acct: String,
        db_pool: &Pool<AsyncPgConnection>,
    ) -> anyhow::Result<Option<Self>> {
        let acct = acct.strip_prefix('@').unwrap_or(&acct);
        let acct_parts = acct.split('@').collect::<Vec<&str>>();
        if acct_parts.len() < 2 {
            return Ok(None);
        }

        let user = users::table
            .filter(users::name.eq(acct_parts[0]))
            .filter(users::instance.eq(acct_parts[1]))
            .first::<Self>(&mut db_pool.get().await?)
            .await;
        match user {
            Ok(user) => Ok(Some(user)),
            Err(NotFound) => Ok(None),
            Err(err) => Err(err.into()),
        }
    }

    pub async fn update(
        &self,
        updated_user: UserUpdate,
        db_pool: &Pool<AsyncPgConnection>,
    ) -> anyhow::Result<()> {
        diesel::update(&self)
            .set(updated_user)
            .execute(&mut db_pool.get().await?)
            .await?;
        Ok(())
    }

    pub async fn posts(
        &self,
        pagination: Pagination,
        actor_id: Option<&DbId>,
        exclude_boosts: bool,
        exclude_replies: bool,
        db_pool: &Pool<AsyncPgConnection>,
    ) -> anyhow::Result<Vec<TimelineEntry>> {
        timelines::get_user_posts(
            &self.id,
            pagination,
            actor_id,
            exclude_boosts,
            exclude_replies,
            db_pool,
        )
        .await
    }

    pub async fn follows_by_id(
        &self,
        user_id: &DbId,
        db_pool: &Pool<AsyncPgConnection>,
    ) -> anyhow::Result<bool> {
        let result = user_followers::table
            .select(sql::<Bool>("true"))
            .filter(user_followers::actor_id.eq(&self.id))
            .filter(user_followers::follower_id.eq(user_id))
            .first::<bool>(&mut db_pool.get().await?)
            .await;
        match result {
            Ok(_) => Ok(true),
            Err(NotFound) => Ok(false),
            Err(err) => Err(err.into()),
        }
    }

    pub async fn follows(
        &self,
        user: &User,
        db_pool: &Pool<AsyncPgConnection>,
    ) -> anyhow::Result<bool> {
        self.follows_by_id(&user.id, db_pool).await
    }

    pub async fn wants_to_follow(
        &self,
        user: &User,
        db_pool: &Pool<AsyncPgConnection>,
    ) -> anyhow::Result<bool> {
        let result = user_follow_requests::table
            .select(sql::<Bool>("true"))
            .filter(user_follow_requests::actor_id.eq(&self.id))
            .filter(user_follow_requests::follower_id.eq(&user.id))
            .first::<bool>(&mut db_pool.get().await?)
            .await;
        match result {
            Ok(_) => Ok(true),
            Err(NotFound) => Ok(false),
            Err(err) => Err(err.into()),
        }
    }

    pub async fn followers(
        &self,
        pagination: Pagination,
        db_pool: &Pool<AsyncPgConnection>,
    ) -> anyhow::Result<Vec<Self>> {
        let query = user_followers::table
            .filter(user_followers::follower_id.eq(&self.id))
            .inner_join(users::dsl::users.on(users::id.eq(user_followers::actor_id)))
            .select(users::all_columns)
            .into_boxed();
        let query = paginate!(query, users::id, pagination);

        Ok(query.load::<Self>(&mut db_pool.get().await?).await?)
    }

    pub async fn following(
        &self,
        pagination: Pagination,
        db_pool: &Pool<AsyncPgConnection>,
    ) -> anyhow::Result<Vec<Self>> {
        let query = user_followers::table
            .filter(user_followers::actor_id.eq(&self.id))
            .inner_join(users::dsl::users.on(users::id.eq(user_followers::follower_id)))
            .select(users::all_columns)
            .into_boxed();
        let query = paginate!(query, users::id, pagination);

        Ok(query.load::<Self>(&mut db_pool.get().await?).await?)
    }

    pub async fn follow_requests(
        &self,
        pagination: Pagination,
        db_pool: &Pool<AsyncPgConnection>,
    ) -> anyhow::Result<Vec<Self>> {
        UserFollowRequest::by_user(&self.id, pagination, db_pool).await
    }

    pub async fn reached_inboxes(
        &self,
        db_pool: &Pool<AsyncPgConnection>,
    ) -> anyhow::Result<Vec<String>> {
        Ok(user_followers::table
            .filter(user_followers::follower_id.eq(&self.id))
            .inner_join(users::dsl::users.on(users::id.eq(user_followers::actor_id)))
            .select(coalesce(users::shared_inbox_uri, users::inbox_uri))
            .distinct()
            .load::<String>(&mut db_pool.get().await?)
            .await?)
    }

    pub async fn liked_posts(
        &self,
        pagination: Pagination,
        db_pool: &Pool<AsyncPgConnection>,
    ) -> anyhow::Result<Vec<Post>> {
        let query = post_like::table
            .filter(post_like::actor_id.eq(&self.id))
            .inner_join(posts::dsl::posts.on(posts::id.eq(post_like::post_id)))
            .select(posts::all_columns)
            .order(post_like::published.desc())
            .into_boxed();
        let query = paginate!(query, posts::id, pagination);

        Ok(query.load::<Post>(&mut db_pool.get().await?).await?)
    }

    pub async fn bookmarked_posts(
        &self,
        pagination: Pagination,
        db_pool: &Pool<AsyncPgConnection>,
    ) -> anyhow::Result<Vec<Post>> {
        let query = bookmarks::table
            .filter(bookmarks::actor_id.eq(&self.id))
            .inner_join(posts::dsl::posts.on(posts::id.eq(bookmarks::post_id)))
            .select(posts::all_columns)
            .order(bookmarks::published.desc())
            .into_boxed();
        let query = paginate!(query, posts::id, pagination);

        Ok(query.load::<Post>(&mut db_pool.get().await?).await?)
    }
}

impl UserUpdate {
    pub fn new() -> Self {
        Self {
            name: None,
            display_name: None,
            bio: None,
            password_encrypted: None,
            admin: None,
            updated: None,
            manually_approves_followers: None,
            is_cat: None,
            bot: None,
        }
    }
}

impl Default for UserUpdate {
    fn default() -> Self {
        Self::new()
    }
}
