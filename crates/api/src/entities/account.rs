use std::sync::Arc;

use chrono::NaiveDateTime;
use db::{models::User, schema::user_followers};
use diesel::{ExpressionMethods, QueryDsl};
use diesel_async::RunQueryDsl;
use serde::Serialize;
use web::AppState;

// TODO: Fully implement https://docs.joinmastodon.org/entities/Account/
#[derive(Clone, Serialize, Debug)]
pub struct Account {
    pub id: String,
    pub url: String,
    pub username: String,
    pub acct: String,
    pub display_name: String,
    pub locked: bool,
    pub created_at: NaiveDateTime,
    pub note: String,
    pub followers_count: u32,
    pub following_count: u32,
}

impl Account {
    pub async fn build(user: User, state: &Arc<AppState>) -> anyhow::Result<Self> {
        let mut conn = state.db_pool.get().await?;

        let followers_count: i64 = user_followers::table
            .filter(user_followers::follower_id.eq(user.id.clone()))
            .count()
            .get_result(&mut conn)
            .await?;
        let following_count: i64 = user_followers::table
            .filter(user_followers::actor_id.eq(user.id.clone()))
            .count()
            .get_result(&mut conn)
            .await?;

        Ok(Self {
            id: user.id.to_string(),
            url: user.ap_id, // TODO: Discuss
            username: user.name.clone(),
            display_name: user.display_name.unwrap_or(user.name.clone()),
            locked: user.manually_approves_followers,
            acct: if user.local {
                user.name
            } else {
                format!("{}@{}", user.name, user.instance)
            },
            created_at: user.published,
            note: user.bio.unwrap_or(String::new()),
            followers_count: followers_count.try_into().unwrap(),
            following_count: following_count.try_into().unwrap(),
        })
    }
}
