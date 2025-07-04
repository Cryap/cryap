use anyhow::anyhow;
use chrono::{DateTime, Utc};
use diesel::{delete, insert_into, prelude::*, result::Error::NotFound};
use diesel_async::{pooled_connection::deadpool::Pool, AsyncPgConnection, RunQueryDsl};

use crate::{
    models::{Post, User},
    schema::post_boost,
    types::{DbId, DbVisibility},
};

#[derive(Queryable, Insertable, AsChangeset, Selectable, Debug, PartialEq, Clone, Eq)]
#[diesel(table_name = post_boost)]
pub struct PostBoost {
    pub id: DbId,
    pub ap_id: String,
    pub post_id: DbId,
    pub actor_id: DbId,
    pub visibility: DbVisibility,
    pub published: DateTime<Utc>,
}

impl PostBoost {
    pub async fn create(boost: Self, db_pool: &Pool<AsyncPgConnection>) -> anyhow::Result<Self> {
        Ok(insert_into(post_boost::table)
            .values(vec![boost])
            .on_conflict((post_boost::actor_id, post_boost::post_id))
            .do_nothing()
            .get_result::<Self>(&mut db_pool.get().await?)
            .await?)
    }

    pub async fn delete(
        actor: &User,
        post: &Post,
        db_pool: &Pool<AsyncPgConnection>,
    ) -> anyhow::Result<bool> {
        let rows_affected = delete(
            post_boost::table
                .filter(post_boost::actor_id.eq(actor.id.clone()))
                .filter(post_boost::post_id.eq(post.id.clone())),
        )
        .execute(&mut db_pool.get().await?)
        .await
        .optional()?;

        Ok(rows_affected == Some(1))
    }

    pub async fn by_id(
        id: &DbId,
        db_pool: &Pool<AsyncPgConnection>,
    ) -> anyhow::Result<Option<Self>> {
        let boost = post_boost::table
            .filter(post_boost::id.eq(id))
            .first::<Self>(&mut db_pool.get().await?)
            .await;
        match boost {
            Ok(boost) => Ok(Some(boost)),
            Err(NotFound) => Ok(None),
            Err(err) => Err(err.into()),
        }
    }

    pub async fn post(&self, db_pool: &Pool<AsyncPgConnection>) -> anyhow::Result<Post> {
        Post::by_id(&self.post_id, db_pool)
            .await?
            .ok_or(anyhow!("This wasn't supposed to happen"))
    }

    pub async fn author(&self, db_pool: &Pool<AsyncPgConnection>) -> anyhow::Result<User> {
        User::by_id(&self.actor_id, db_pool)
            .await?
            .ok_or(anyhow!("This wasn't supposed to happen"))
    }
}
