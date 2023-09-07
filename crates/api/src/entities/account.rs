use std::sync::Arc;

use chrono::NaiveDateTime;
use db::{models::User, schema::user_followers, types::DbVisibility};
use diesel::{ExpressionMethods, QueryDsl};
use diesel_async::RunQueryDsl;
use futures::future::join_all;
use serde::Serialize;
use web::AppState;

#[derive(Clone, Serialize, Debug)]
pub struct AccountSource {
    note: String,
    fields: Vec<()>,
    privacy: DbVisibility,
    sensitive: bool,
    language: String,
    follow_requests_count: u32,
}

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
    pub source: Option<AccountSource>,

    pub header: String,
    pub avatar: String,
    pub header_static: String,
    pub avatar_static: String,
}

impl Account {
    pub async fn build(
        user: User,
        state: &Arc<AppState>,
        with_source: bool,
    ) -> anyhow::Result<Self> {
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
            note: user.bio.clone().unwrap_or(String::new()),
            followers_count: followers_count.try_into().unwrap(),
            following_count: following_count.try_into().unwrap(),

            source: match with_source {
                true => Some(AccountSource {
                    sensitive: false,
                    note: user.bio.unwrap_or(String::new()),
                    fields: vec![],
                    privacy: DbVisibility::Public,
                    language: "en".to_string(),
                    follow_requests_count: 0, // TODO
                }),
                false => None,
            },

            header: "https://http.cat/images/404.jpg".to_string(), // TODO: Media
            header_static: "https://http.cat/images/404.jpg".to_string(),
            avatar: "https://http.cat/images/404.jpg".to_string(),
            avatar_static: "https://http.cat/images/404.jpg".to_string(),
        })
    }

    pub async fn build_from_vec(
        users: Vec<User>,
        state: &Arc<AppState>,
    ) -> anyhow::Result<Vec<Self>> {
        join_all(
            users
                .into_iter()
                .map(|user| async { Self::build(user, state, false).await }),
        )
        .await
        .into_iter()
        .collect()
    }
}
