use chrono::{DateTime, Utc};
use diesel::{
    dsl::sql,
    prelude::*,
    result::Error::NotFound,
    select, sql_query,
    sql_types::{Bool, Bpchar, Varchar},
};
use diesel_async::{pooled_connection::deadpool::Pool, AsyncPgConnection, RunQueryDsl};

use crate::{
    common::timelines::{self, TimelineEntry},
    models::{user_follow_request::UserFollowRequest, Post},
    paginate,
    pagination::Pagination,
    schema::{
        bookmarks, post_boost, post_like, posts, user_follow_requests, user_followers, users,
    },
    types::DbId,
    utils::coalesce,
};

#[derive(Queryable, Identifiable, Selectable, Debug, PartialEq, Clone, Eq)]
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
    /// Updated by database triggers defined in `../../migrations/2025-06-29-211313_save_user_stats/up.sql`
    pub followers_count: i32,
    /// Updated by database triggers defined in `../../migrations/2025-06-29-211313_save_user_stats/up.sql`
    pub following_count: i32,
    /// Updated by database triggers defined in `../../migrations/2025-06-29-211313_save_user_stats/up.sql`
    pub follow_requests_count: i32,
    /// Updated by database triggers defined in `../../migrations/2025-06-29-211313_save_user_stats/up.sql`
    pub posts_count: i32,
    /// Updated by database triggers defined in `../../migrations/2025-06-29-211313_save_user_stats/up.sql`
    pub last_post_published: Option<DateTime<Utc>>,
}

#[derive(Clone, Insertable, AsChangeset)]
#[diesel(table_name = users)]
pub struct UserInsert {
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

pub struct UserRelationship {
    pub following: bool,
    pub followed_by: bool,
    pub wants_to_follow: bool,
}

#[derive(QueryableByName)]
struct FollowIdResult {
    #[diesel(sql_type = Varchar)]
    coalesce: String,
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

    pub async fn by_ids(
        ids: Vec<&DbId>,
        db_pool: &Pool<AsyncPgConnection>,
    ) -> anyhow::Result<Vec<Option<Self>>> {
        let users = users::table
            .filter(users::id.eq_any(&ids))
            .load::<Self>(&mut db_pool.get().await?)
            .await?;
        Ok(ids
            .into_iter()
            .map(|id| users.iter().find(|user| user.id == *id).cloned())
            .collect())
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

    pub async fn relationship(
        &self,
        user: &User,
        db_pool: &Pool<AsyncPgConnection>,
    ) -> anyhow::Result<UserRelationship> {
        let (following, followed_by, wants_to_follow): (Option<bool>, Option<bool>, Option<bool>) =
            select((
                user_followers::table
                    .select(sql::<Bool>("true"))
                    .filter(user_followers::actor_id.eq(&self.id))
                    .filter(user_followers::follower_id.eq(&user.id))
                    .single_value(),
                user_followers::table
                    .select(sql::<Bool>("true"))
                    .filter(user_followers::actor_id.eq(&user.id))
                    .filter(user_followers::follower_id.eq(&self.id))
                    .single_value(),
                user_follow_requests::table
                    .select(sql::<Bool>("true"))
                    .filter(user_follow_requests::actor_id.eq(&self.id))
                    .filter(user_follow_requests::follower_id.eq(&user.id))
                    .single_value(),
            ))
            .first(&mut db_pool.get().await?)
            .await?;

        Ok(UserRelationship {
            following: following.unwrap_or_default(),
            followed_by: followed_by.unwrap_or_default(),
            wants_to_follow: wants_to_follow.unwrap_or_default(),
        })
    }

    pub async fn followers(
        &self,
        pagination: Pagination,
        db_pool: &Pool<AsyncPgConnection>,
    ) -> anyhow::Result<Vec<Self>> {
        let query = user_followers::table
            .filter(user_followers::follower_id.eq(&self.id))
            .inner_join(users::table.on(users::id.eq(user_followers::actor_id)))
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
            .inner_join(users::table.on(users::id.eq(user_followers::follower_id)))
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

    pub async fn follow_id(
        &self,
        user: &User,
        db_pool: &Pool<AsyncPgConnection>,
    ) -> anyhow::Result<Option<String>> {
        Ok(sql_query(
            "
            SELECT COALESCE(
                user_followers.ap_id, user_follow_requests.ap_id
            ) 
            FROM user_followers
            FULL JOIN user_follow_requests ON user_followers.actor_id = user_follow_requests.actor_id 
                AND user_followers.follower_id = user_follow_requests.follower_id 
            WHERE user_followers.actor_id = $1 
                AND user_followers.follower_id = $2;
            ",
        )
        .bind::<Bpchar, _>(self.id.clone())
        .bind::<Bpchar, _>(user.id.clone())
        .load::<FollowIdResult>(&mut db_pool.get().await?)
        .await?
        .into_iter()
        .next()
        .map(|result| result.coalesce))
    }

    pub async fn reached_inboxes(
        &self,
        db_pool: &Pool<AsyncPgConnection>,
    ) -> anyhow::Result<Vec<String>> {
        Ok(user_followers::table
            .filter(user_followers::follower_id.eq(&self.id))
            .filter(users::local.eq(false))
            .inner_join(users::table.on(users::id.eq(user_followers::actor_id)))
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
            .inner_join(posts::table.on(posts::id.eq(post_like::post_id)))
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
            .inner_join(posts::table.on(posts::id.eq(bookmarks::post_id)))
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
