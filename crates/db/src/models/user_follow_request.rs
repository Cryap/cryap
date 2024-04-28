use chrono::{DateTime, Utc};
use diesel::{delete, insert_into, prelude::*, result::Error::NotFound};
use diesel_async::{pooled_connection::deadpool::Pool, AsyncPgConnection, RunQueryDsl};

use crate::{
    models::User,
    paginate,
    pagination::Pagination,
    schema::{user_follow_requests, users},
    types::DbId,
};

#[derive(Queryable, Selectable, Debug, PartialEq, Clone, Eq)]
#[diesel(table_name = user_follow_requests)]
pub struct UserFollowRequest {
    pub actor_id: DbId,
    pub follower_id: DbId,
    pub ap_id: Option<String>,
    pub published: DateTime<Utc>,
}

#[derive(Clone, Insertable, AsChangeset)]
#[diesel(table_name = user_follow_requests)]
pub struct UserFollowRequestsInsert {
    pub actor_id: DbId,
    pub follower_id: DbId,
    pub ap_id: Option<String>,
}

impl UserFollowRequest {
    pub async fn create(
        actor: &User,
        follower: &User,
        ap_id: Option<String>,
        db_pool: &Pool<AsyncPgConnection>,
    ) -> anyhow::Result<bool> {
        let rows_affected = insert_into(user_follow_requests::table)
            .values(vec![UserFollowRequestsInsert {
                actor_id: actor.id.clone(),
                follower_id: follower.id.clone(),
                ap_id,
            }])
            .on_conflict((
                user_follow_requests::actor_id,
                user_follow_requests::follower_id,
            ))
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
                user_follow_requests::table
                    .filter(user_follow_requests::actor_id.eq(actor.id.clone()))
                    .filter(user_follow_requests::follower_id.eq(follower.id.clone()))
                    .filter(user_follow_requests::ap_id.eq(ap_id)),
            )
            .execute(&mut db_pool.get().await?)
            .await
            .optional()?
        } else {
            delete(
                user_follow_requests::table
                    .filter(user_follow_requests::actor_id.eq(actor.id.clone()))
                    .filter(user_follow_requests::follower_id.eq(follower.id.clone())),
            )
            .execute(&mut db_pool.get().await?)
            .await
            .optional()?
        };

        Ok(rows_affected == Some(1))
    }

    pub async fn by_user(
        user_id: &DbId,
        pagination: Pagination,
        db_pool: &Pool<AsyncPgConnection>,
    ) -> anyhow::Result<Vec<User>> {
        let query = user_follow_requests::table
            .filter(user_follow_requests::follower_id.eq(user_id))
            .inner_join(users::table.on(users::id.eq(user_follow_requests::actor_id)))
            .select(users::all_columns)
            .into_boxed();
        let query = paginate!(query, users::id, pagination);

        Ok(query.load::<User>(&mut db_pool.get().await?).await?)
    }

    pub async fn by_actor_and_follower(
        actor: &User,
        follower: &User,
        db_pool: &Pool<AsyncPgConnection>,
    ) -> anyhow::Result<Option<Self>> {
        let request = user_follow_requests::table
            .filter(user_follow_requests::actor_id.eq(actor.id.clone()))
            .filter(user_follow_requests::follower_id.eq(follower.id.clone()))
            .first::<Self>(&mut db_pool.get().await?)
            .await;
        match request {
            Ok(request) => Ok(Some(request)),
            Err(NotFound) => Ok(None),
            Err(err) => Err(err.into()),
        }
    }
}
