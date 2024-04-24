use chrono::{DateTime, Utc};
use diesel::{delete, insert_into, prelude::*};
use diesel_async::{pooled_connection::deadpool::Pool, AsyncPgConnection, RunQueryDsl};

use crate::{models::User, schema::user_followers, types::DbId};

#[derive(Queryable, Selectable, Debug, PartialEq, Clone, Eq)]
#[diesel(table_name = user_followers)]
pub struct UserFollower {
    pub actor_id: DbId,
    pub follower_id: DbId,
    pub ap_id: Option<String>,
    pub published: DateTime<Utc>,
}

#[derive(Clone, Insertable, AsChangeset)]
#[diesel(table_name = user_followers)]
pub struct UserFollowersInsert {
    pub actor_id: DbId,
    pub follower_id: DbId,
    pub ap_id: Option<String>,
}

impl UserFollower {
    pub async fn create(
        actor: &User,
        follower: &User,
        ap_id: Option<String>,
        db_pool: &Pool<AsyncPgConnection>,
    ) -> anyhow::Result<bool> {
        let rows_affected = insert_into(user_followers::table)
            .values(vec![UserFollowersInsert {
                actor_id: actor.id.clone(),
                follower_id: follower.id.clone(),
                ap_id,
            }])
            .on_conflict((user_followers::actor_id, user_followers::follower_id))
            .do_nothing()
            .execute(&mut db_pool.get().await?)
            .await
            .optional()?;

        Ok(rows_affected == Some(1))
    }

    pub async fn delete(
        actor: &User,
        follower: &User,
        ap_id: Option<String>,
        db_pool: &Pool<AsyncPgConnection>,
    ) -> anyhow::Result<bool> {
        let rows_affected = if let Some(ap_id) = ap_id {
            delete(
                user_followers::table
                    .filter(user_followers::actor_id.eq(actor.id.clone()))
                    .filter(user_followers::follower_id.eq(follower.id.clone()))
                    .filter(user_followers::ap_id.eq(ap_id)),
            )
            .execute(&mut db_pool.get().await?)
            .await
            .optional()?
        } else {
            delete(
                user_followers::table
                    .filter(user_followers::actor_id.eq(actor.id.clone()))
                    .filter(user_followers::follower_id.eq(follower.id.clone())),
            )
            .execute(&mut db_pool.get().await?)
            .await
            .optional()?
        };

        Ok(rows_affected == Some(1))
    }
}
