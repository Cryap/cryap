use diesel::{dsl::sql, prelude::*, result::Error::NotFound, sql_types::Bool};
use diesel_async::{pooled_connection::deadpool::Pool, AsyncPgConnection, RunQueryDsl};

use crate::{
    pagination::Pagination,
    schema::{user_follow_requests, user_followers, users},
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
    pub published: chrono::NaiveDateTime,
    pub updated: Option<chrono::NaiveDateTime>,
    pub manually_approves_followers: bool,
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

    pub async fn by_name(
        name: &str,
        db_pool: &Pool<AsyncPgConnection>,
    ) -> anyhow::Result<Option<Self>> {
        let user = users::table
            .filter(users::name.eq(name.to_string()))
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
        let acct = if acct.starts_with('@') {
            &acct[1..]
        } else {
            &acct
        };

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

    pub async fn follows(
        &self,
        user: &User,
        db_pool: &Pool<AsyncPgConnection>,
    ) -> anyhow::Result<bool> {
        let result = user_followers::table
            .select(sql::<Bool>("true"))
            .filter(user_followers::actor_id.eq(&self.id))
            .filter(user_followers::follower_id.eq(&user.id))
            .first::<bool>(&mut db_pool.get().await?)
            .await;
        match result {
            Ok(_) => Ok(true),
            Err(NotFound) => Ok(false),
            Err(err) => Err(err.into()),
        }
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
        let query = match pagination {
            Pagination::MaxId(id, limit) => query.filter(users::id.gt(id)).limit(limit.into()),
            Pagination::MinId(id, limit) => query.filter(users::id.lt(id)).limit(limit.into()),
            Pagination::None(limit) => query.limit(limit.into()),
        };

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
        let query = match pagination {
            Pagination::MaxId(id, limit) => query.filter(users::id.gt(id)).limit(limit.into()),
            Pagination::MinId(id, limit) => query.filter(users::id.lt(id)).limit(limit.into()),
            Pagination::None(limit) => query.limit(limit.into()),
        };

        Ok(query.load::<Self>(&mut db_pool.get().await?).await?)
    }

    pub async fn following_inboxes(
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
}
