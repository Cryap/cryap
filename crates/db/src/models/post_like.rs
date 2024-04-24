use chrono::{DateTime, Utc};
use diesel::{delete, insert_into, prelude::*, result::Error::NotFound};
use diesel_async::{pooled_connection::deadpool::Pool, AsyncPgConnection, RunQueryDsl};

use crate::{
    models::{Post, User},
    schema::post_like,
    types::DbId,
};

#[derive(Queryable, Insertable, AsChangeset, Selectable, Debug, PartialEq, Clone, Eq)]
#[diesel(table_name = post_like)]
pub struct PostLike {
    pub ap_id: Option<String>,
    pub post_id: DbId,
    pub actor_id: DbId,
    pub published: DateTime<Utc>,
}

impl PostLike {
    pub async fn create(
        ap_id: Option<String>,
        post: &Post,
        actor: &User,
        db_pool: &Pool<AsyncPgConnection>,
    ) -> anyhow::Result<bool> {
        let rows_affected = insert_into(post_like::table)
            .values(vec![PostLike {
                ap_id,
                post_id: post.id.clone(),
                actor_id: actor.id.clone(),
                published: Utc::now(),
            }])
            .on_conflict((post_like::actor_id, post_like::post_id))
            .do_nothing()
            .execute(&mut db_pool.get().await?)
            .await
            .optional()?;

        Ok(rows_affected == Some(1))
    }

    pub async fn delete(
        ap_id: Option<String>,
        post: &Post,
        actor: &User,
        db_pool: &Pool<AsyncPgConnection>,
    ) -> anyhow::Result<bool> {
        let rows_affected = if let Some(ap_id) = ap_id {
            delete(
                post_like::table
                    .filter(post_like::ap_id.eq(ap_id))
                    .filter(post_like::post_id.eq(post.id.clone()))
                    .filter(post_like::actor_id.eq(actor.id.clone())),
            )
            .execute(&mut db_pool.get().await?)
            .await
            .optional()?
        } else {
            delete(
                post_like::table
                    .filter(post_like::post_id.eq(post.id.clone()))
                    .filter(post_like::actor_id.eq(actor.id.clone())),
            )
            .execute(&mut db_pool.get().await?)
            .await
            .optional()?
        };

        Ok(rows_affected == Some(1))
    }

    pub async fn by_post_and_actor(
        post: &Post,
        actor: &User,
        db_pool: &Pool<AsyncPgConnection>,
    ) -> anyhow::Result<Option<Self>> {
        let like = post_like::table
            .filter(post_like::post_id.eq(post.id.clone()))
            .filter(post_like::actor_id.eq(actor.id.clone()))
            .first::<Self>(&mut db_pool.get().await?)
            .await;
        match like {
            Ok(like) => Ok(Some(like)),
            Err(NotFound) => Ok(None),
            Err(err) => Err(err.into()),
        }
    }
}
