use anyhow::anyhow;
use chrono::{DateTime, Utc};
use diesel::{delete, dsl::not, insert_into, prelude::*, result::Error::NotFound};
use diesel_async::{pooled_connection::deadpool::Pool, AsyncPgConnection, RunQueryDsl};

use crate::{
    models::{Post, User},
    paginate,
    pagination::Pagination,
    schema::notifications,
    types::{DbId, DbNotificationType},
};

#[derive(Queryable, Identifiable, Insertable, Selectable, Debug, PartialEq, Clone, Eq)]
#[diesel(table_name = notifications)]
pub struct Notification {
    pub id: DbId,
    pub actor_id: DbId,
    pub receiver_id: DbId,
    pub post_id: Option<DbId>,
    pub notification_type: DbNotificationType,
    pub published: DateTime<Utc>,
}

impl Notification {
    pub async fn create_by_ids(
        actor_id: DbId,
        receiver_id: DbId,
        post_id: Option<DbId>,
        notification_type: DbNotificationType,
        db_pool: &Pool<AsyncPgConnection>,
    ) -> anyhow::Result<Self> {
        let notification = Notification {
            id: DbId::default(),
            actor_id,
            receiver_id,
            post_id,
            notification_type,
            published: Utc::now(),
        };

        Ok(insert_into(notifications::table)
            .values(notification)
            .on_conflict(notifications::id)
            .do_nothing()
            .get_result::<Notification>(&mut db_pool.get().await?)
            .await?)
    }

    pub async fn create(
        actor: &User,
        receiver: &User,
        post: Option<&Post>,
        notification_type: DbNotificationType,
        db_pool: &Pool<AsyncPgConnection>,
    ) -> anyhow::Result<Self> {
        Self::create_by_ids(
            actor.id.clone(),
            receiver.id.clone(),
            post.map(|post| post.id.clone()),
            notification_type,
            db_pool,
        )
        .await
    }

    pub async fn delete_by_ids(
        actor_id: &DbId,
        receiver_id: &DbId,
        post_id: Option<&DbId>,
        notification_type: DbNotificationType,
        db_pool: &Pool<AsyncPgConnection>,
    ) -> anyhow::Result<()> {
        if let Some(post_id) = post_id {
            delete(
                notifications::table
                    .filter(notifications::actor_id.eq(actor_id))
                    .filter(notifications::receiver_id.eq(receiver_id))
                    .filter(notifications::notification_type.eq(&notification_type))
                    .filter(notifications::post_id.eq(post_id)),
            )
            .execute(&mut db_pool.get().await?)
            .await?;
        } else {
            delete(
                notifications::table
                    .filter(notifications::actor_id.eq(actor_id))
                    .filter(notifications::receiver_id.eq(receiver_id))
                    .filter(notifications::notification_type.eq(&notification_type))
                    .filter(notifications::post_id.is_null()),
            )
            .execute(&mut db_pool.get().await?)
            .await?;
        }

        Ok(())
    }

    pub async fn delete(
        actor: &User,
        receiver: &User,
        post: Option<&Post>,
        notification_type: DbNotificationType,
        db_pool: &Pool<AsyncPgConnection>,
    ) -> anyhow::Result<()> {
        Self::delete_by_ids(
            &actor.id,
            &receiver.id,
            post.map(|post| &post.id),
            notification_type,
            db_pool,
        )
        .await
    }

    pub async fn by_id(
        id: &DbId,
        db_pool: &Pool<AsyncPgConnection>,
    ) -> anyhow::Result<Option<Self>> {
        let notification = notifications::table
            .filter(notifications::id.eq(id))
            .first::<Self>(&mut db_pool.get().await?)
            .await;
        match notification {
            Ok(notification) => Ok(Some(notification)),
            Err(NotFound) => Ok(None),
            Err(err) => Err(err.into()),
        }
    }

    pub async fn get_for_user(
        user_id: &DbId,
        pagination: Pagination,
        types: Option<Vec<DbNotificationType>>,
        exclude_types: Option<Vec<DbNotificationType>>,
        by_account_id: Option<DbId>,
        db_pool: &Pool<AsyncPgConnection>,
    ) -> anyhow::Result<Vec<Self>> {
        let mut query = notifications::table
            .filter(notifications::receiver_id.eq(user_id))
            .select(notifications::all_columns)
            .order(notifications::published.desc())
            .into_boxed();

        if let Some(types) = types {
            query = query.filter(notifications::notification_type.eq_any(types));
        }

        if let Some(exclude_types) = exclude_types {
            query = query.filter(not(notifications::notification_type.eq_any(exclude_types)));
        }

        if let Some(by_account_id) = by_account_id {
            query = query.filter(notifications::actor_id.eq(by_account_id));
        }

        query = paginate!(query, notifications::id, pagination);

        Ok(query.load::<Self>(&mut db_pool.get().await?).await?)
    }

    pub async fn clear_for_user(
        user_id: &DbId,
        db_pool: &Pool<AsyncPgConnection>,
    ) -> anyhow::Result<()> {
        delete(notifications::table.filter(notifications::receiver_id.eq(user_id)))
            .execute(&mut db_pool.get().await?)
            .await?;
        Ok(())
    }

    pub async fn actor(&self, db_pool: &Pool<AsyncPgConnection>) -> anyhow::Result<User> {
        User::by_id(&self.actor_id, db_pool)
            .await?
            .ok_or(anyhow!("This wasn't supposed to happen"))
    }

    pub async fn receiver(&self, db_pool: &Pool<AsyncPgConnection>) -> anyhow::Result<User> {
        User::by_id(&self.receiver_id, db_pool)
            .await?
            .ok_or(anyhow!("This wasn't supposed to happen"))
    }

    pub async fn post(&self, db_pool: &Pool<AsyncPgConnection>) -> anyhow::Result<Option<Post>> {
        Ok(if let Some(post_id) = &self.post_id {
            Some(
                Post::by_id(post_id, db_pool)
                    .await?
                    .ok_or(anyhow!("This wasn't supposed to happen"))?,
            )
        } else {
            None
        })
    }

    pub async fn dismiss(&self, db_pool: &Pool<AsyncPgConnection>) -> anyhow::Result<()> {
        delete(notifications::table.filter(notifications::id.eq(&self.id)))
            .execute(&mut db_pool.get().await?)
            .await?;
        Ok(())
    }
}
