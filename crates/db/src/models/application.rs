use chrono::Utc;
use diesel::{insert_into, prelude::*, result::Error::NotFound};
use diesel_async::{pooled_connection::deadpool::Pool, AsyncPgConnection, RunQueryDsl};

use crate::{schema::applications, types::DbId, utils::random_string};

#[derive(Queryable, Identifiable, Insertable, Selectable, Debug, PartialEq, Clone, Eq)]
#[diesel(table_name = applications)]
pub struct Application {
    pub id: DbId,
    pub name: String,
    pub website: Option<String>,
    pub redirect_url: String,
    pub client_id: String,
    pub client_secret: String,
    pub published: chrono::NaiveDateTime,
}

impl Application {
    pub async fn create(
        name: String,
        website: Option<String>,
        redirect_url: String,
        db_pool: &Pool<AsyncPgConnection>,
    ) -> Result<Self, anyhow::Error> {
        let application = Application {
            id: DbId::default(),
            name,
            website,
            redirect_url,
            client_id: random_string(32),
            client_secret: random_string(32),
            published: Utc::now().naive_utc(),
        };

        Ok(insert_into(applications::table)
            .values(application.clone())
            .get_result::<Application>(&mut db_pool.get().await?)
            .await?)
    }

    pub async fn by_id(
        id: &DbId,
        db_pool: &Pool<AsyncPgConnection>,
    ) -> Result<Option<Self>, anyhow::Error> {
        let application = applications::table
            .filter(applications::id.eq(id))
            .first::<Self>(&mut db_pool.get().await?)
            .await;
        match application {
            Ok(application) => Ok(Some(application)),
            Err(NotFound) => Ok(None),
            Err(err) => Err(err.into()),
        }
    }

    pub async fn by_client_id(
        client_id: &str,
        db_pool: &Pool<AsyncPgConnection>,
    ) -> Result<Option<Self>, anyhow::Error> {
        let application = applications::table
            .filter(applications::client_id.eq(client_id.to_string()))
            .first::<Self>(&mut db_pool.get().await?)
            .await;
        match application {
            Ok(application) => Ok(Some(application)),
            Err(NotFound) => Ok(None),
            Err(err) => Err(err.into()),
        }
    }
}
