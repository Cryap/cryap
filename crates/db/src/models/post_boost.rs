use anyhow::anyhow;
use diesel::{prelude::*, result::Error::NotFound};
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
    pub published: chrono::NaiveDateTime,
}

impl PostBoost {
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
