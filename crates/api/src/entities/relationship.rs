use db::models::{PrivateNote, User};
use diesel_async::{pooled_connection::deadpool::Pool, AsyncPgConnection};
use serde::Serialize;

// TODO: Fully implement https://docs.joinmastodon.org/entities/Relationship/
#[derive(Serialize, Debug)]
pub struct Relationship {
    pub id: String,
    pub following: bool,
    pub followed_by: bool,
    pub requested: bool,
    pub note: String,
}

impl Relationship {
    pub async fn build(
        by: &User,
        to: &User,
        db_pool: &Pool<AsyncPgConnection>,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            id: to.id.to_string(),
            following: by.follows(to, db_pool).await?,
            followed_by: to.follows(by, db_pool).await?,
            requested: by.wants_to_follow(to, db_pool).await?,
            note: PrivateNote::get(by, to, db_pool).await?.unwrap_or_default(),
        })
    }
}
