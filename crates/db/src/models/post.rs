use diesel::{prelude::*, result::Error::NotFound};
use diesel_async::{pooled_connection::deadpool::Pool, AsyncPgConnection, RunQueryDsl};

use crate::schema::post_mention;
use crate::{
    schema::posts,
    types::{DbId, DbVisibility},
};

#[derive(
    Queryable, Insertable, Identifiable, AsChangeset, Selectable, Debug, PartialEq, Clone, Eq,
)]
#[diesel(table_name = posts)]
pub struct Post {
    pub id: DbId,
    pub author: DbId,
    pub ap_id: String,
    pub local_only: bool,
    pub content_warning: Option<String>,
    pub content: String,
    pub sensitive: bool,
    pub in_reply: Option<DbId>,
    pub published: chrono::NaiveDateTime,
    pub updated: Option<chrono::NaiveDateTime>,
    pub url: String,
    pub quote: Option<DbId>,
    pub visibility: DbVisibility,
}

impl Post {
    pub async fn by_id(
        id: &DbId,
        db_pool: &Pool<AsyncPgConnection>,
    ) -> Result<Option<Self>, anyhow::Error> {
        let post = posts::table
            .filter(posts::id.eq(id))
            .first::<Self>(&mut db_pool.get().await?)
            .await;
        match post {
            Ok(post) => Ok(Some(post)),
            Err(NotFound) => Ok(None),
            Err(err) => Err(err.into()),
        }
    }
}

#[derive(Queryable, Insertable, AsChangeset, Selectable, Debug, PartialEq, Clone, Eq)]
#[diesel(table_name = post_mention)]
pub struct PostMention {
    pub id: DbId,
    pub post_id: DbId,
    pub mentioned_user_id: DbId,
}
