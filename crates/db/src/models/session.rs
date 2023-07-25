use anyhow::anyhow;
use chrono::Utc;
use diesel::{insert_into, prelude::*, result::Error::NotFound};
use diesel_async::{pooled_connection::deadpool::Pool, AsyncPgConnection, RunQueryDsl};

use crate::{models::User, schema::sessions, types::DbId, utils::random_string};

#[derive(Queryable, Identifiable, Insertable, Selectable, Debug, PartialEq, Clone, Eq)]
#[diesel(table_name = sessions)]
pub struct Session {
    pub id: DbId,
    pub token: String,
    pub user_id: DbId,
    pub published: chrono::NaiveDateTime,
}

impl Session {
    pub async fn create(
        user_id: DbId,
        db_pool: &Pool<AsyncPgConnection>,
    ) -> Result<Self, anyhow::Error> {
        let session = Session {
            id: DbId::default(),
            token: random_string(60),
            user_id,
            published: Utc::now().naive_utc(),
        };

        Ok(insert_into(sessions::table)
            .values(session.clone())
            .get_result::<Session>(&mut db_pool.get().await?)
            .await?)
    }

    pub async fn by_token(
        token: &str,
        db_pool: &Pool<AsyncPgConnection>,
    ) -> Result<Option<Self>, anyhow::Error> {
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

    pub async fn user(&self, db_pool: &Pool<AsyncPgConnection>) -> Result<User, anyhow::Error> {
        User::by_id(&self.user_id, db_pool)
            .await?
            .ok_or(anyhow!("This wasn't supposed to happen"))
    }
}
