use anyhow::anyhow;
use diesel::{dsl::sql, prelude::*, result::Error::NotFound, sql_types::Bool};
use diesel_async::{pooled_connection::deadpool::Pool, AsyncPgConnection, RunQueryDsl};

use crate::{
    models::{PostBoost, User},
    schema::{post_boost, post_like, post_mention, posts},
    types::{DbId, DbVisibility},
};

#[derive(
    Queryable, Insertable, Identifiable, AsChangeset, Selectable, Debug, PartialEq, Clone, Eq,
)]
#[diesel(table_name = posts)]
pub struct Post {
    pub id: DbId,
    pub author: DbId,
    pub ap_id: String,
    pub local_only: bool,
    pub content_warning: Option<String>,
    pub content: String,
    pub sensitive: bool,
    pub in_reply: Option<DbId>,
    pub published: chrono::NaiveDateTime,
    pub updated: Option<chrono::NaiveDateTime>,
    pub url: String,
    pub quote: Option<DbId>,
    pub visibility: DbVisibility,
}

impl Post {
    pub async fn by_id(
        id: &DbId,
        db_pool: &Pool<AsyncPgConnection>,
    ) -> anyhow::Result<Option<Self>> {
        let post = posts::table
            .filter(posts::id.eq(id))
            .first::<Self>(&mut db_pool.get().await?)
            .await;
        match post {
            Ok(post) => Ok(Some(post)),
            Err(NotFound) => Ok(None),
            Err(err) => Err(err.into()),
        }
    }

    pub async fn author(&self, db_pool: &Pool<AsyncPgConnection>) -> anyhow::Result<User> {
        User::by_id(&self.author, db_pool)
            .await?
            .ok_or(anyhow!("This wasn't supposed to happen"))
    }

    pub async fn liked_by(
        &self,
        user: &User,
        db_pool: &Pool<AsyncPgConnection>,
    ) -> anyhow::Result<bool> {
        let result = post_like::table
            .select(sql::<Bool>("true"))
            .filter(post_like::post_id.eq(&self.id))
            .filter(post_like::actor_id.eq(&user.id))
            .first::<bool>(&mut db_pool.get().await?)
            .await;
        match result {
            Ok(_) => Ok(true),
            Err(NotFound) => Ok(false),
            Err(err) => Err(err.into()),
        }
    }

    pub async fn boost_by(
        &self,
        user: &User,
        db_pool: &Pool<AsyncPgConnection>,
    ) -> anyhow::Result<Option<PostBoost>> {
        let boost = post_boost::table
            .filter(post_boost::post_id.eq(&self.id))
            .filter(post_boost::actor_id.eq(&user.id))
            .first::<PostBoost>(&mut db_pool.get().await?)
            .await;
        match boost {
            Ok(boost) => Ok(Some(boost)),
            Err(NotFound) => Ok(None),
            Err(err) => Err(err.into()),
        }
    }
}

#[derive(Queryable, Insertable, AsChangeset, Selectable, Debug, PartialEq, Clone, Eq)]
#[diesel(table_name = post_mention)]
pub struct PostMention {
    pub id: DbId,
    pub post_id: DbId,
    pub mentioned_user_id: DbId,
}
