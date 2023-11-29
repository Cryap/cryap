use anyhow::anyhow;
use chrono::{DateTime, Utc};
use diesel::{delete, insert_into, prelude::*, result::Error::NotFound};
use diesel_async::{pooled_connection::deadpool::Pool, AsyncPgConnection, RunQueryDsl};

use crate::{
    models::{Application, User},
    schema::sessions,
    types::DbId,
    utils::random_string,
};

#[derive(Queryable, Identifiable, Insertable, Selectable, Debug, PartialEq, Clone, Eq)]
#[diesel(table_name = sessions)]
pub struct Session {
    pub id: DbId,
    pub token: String,
    pub user_id: DbId,
    pub published: DateTime<Utc>,
    pub application_id: Option<DbId>,
}

impl Session {
    pub async fn create(
        user_id: DbId,
        application_id: Option<DbId>,
        db_pool: &Pool<AsyncPgConnection>,
    ) -> anyhow::Result<Self> {
        let session = Session {
            id: DbId::default(),
            token: random_string(60),
            user_id,
            published: Utc::now(),
            application_id,
        };

        Ok(insert_into(sessions::table)
            .values(session.clone())
            .get_result::<Session>(&mut db_pool.get().await?)
            .await?)
    }

    pub async fn by_token(
        token: &str,
        db_pool: &Pool<AsyncPgConnection>,
    ) -> anyhow::Result<Option<Self>> {
        let session = sessions::table
            .filter(sessions::token.eq(token.to_string()))
            .first::<Self>(&mut db_pool.get().await?)
            .await;
        match session {
            Ok(session) => Ok(Some(session)),
            Err(NotFound) => Ok(None),
            Err(err) => Err(err.into()),
        }
    }

    pub async fn user(&self, db_pool: &Pool<AsyncPgConnection>) -> anyhow::Result<User> {
        User::by_id(&self.user_id, db_pool)
            .await?
            .ok_or(anyhow!("This wasn't supposed to happen"))
    }

    pub async fn application(
        &self,
        db_pool: &Pool<AsyncPgConnection>,
    ) -> anyhow::Result<Option<Application>> {
        match &self.application_id {
            Some(application_id) => Ok(Some(
                Application::by_id(application_id, db_pool)
                    .await?
                    .ok_or(anyhow!("This wasn't supposed to happen"))?,
            )),
            None => Ok(None),
        }
    }

    pub async fn delete(&self, db_pool: &Pool<AsyncPgConnection>) -> anyhow::Result<()> {
        delete(sessions::table.filter(sessions::id.eq(&self.id)))
            .execute(&mut db_pool.get().await?)
            .await?;
        Ok(())
    }
}
