use diesel::{prelude::*, result::Error::NotFound};
use diesel_async::{pooled_connection::deadpool::Pool, AsyncPgConnection, RunQueryDsl};

use crate::{schema::users, types::DbId};

#[derive(Queryable, Identifiable, Selectable, Insertable, AsChangeset, Debug, PartialEq, Clone, Eq)]
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
}

impl User {
    pub async fn by_id(
        id: &DbId,
        db_pool: &Pool<AsyncPgConnection>,
    ) -> Result<Option<Self>, anyhow::Error> {
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
    ) -> Result<Option<Self>, anyhow::Error> {
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
    ) -> Result<Option<Self>, anyhow::Error> {
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
    ) -> Result<Option<Self>, anyhow::Error> {
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
}
