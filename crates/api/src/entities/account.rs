use std::sync::Arc;

use chrono::NaiveDateTime;
use db::{
    models::User,
    schema::{user_follow_requests, user_followers},
    types::DbVisibility,
};
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
    pub bot: bool,
    pub created_at: NaiveDateTime,
    pub note: String,
    pub followers_count: u32,
    pub following_count: u32,
    pub source: Option<AccountSource>,

    pub header: String,
    pub avatar: String,
    pub header_static: String,
    pub avatar_static: String,

    pub is_cat: bool,
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
        let follow_requests_count: i64 = match with_source {
            true => {
                user_follow_requests::table
                    .filter(user_follow_requests::follower_id.eq(user.id.clone()))
                    .count()
                    .get_result(&mut conn)
                    .await?
            },
            false => 0,
        };

        Ok(Self {
            id: user.id.to_string(),
            url: user.ap_id, // TODO: Discuss
            username: user.name.clone(),
            display_name: user.display_name.unwrap_or(user.name.clone()),
            locked: user.manually_approves_followers,
            bot: user.bot,
            acct: if user.local {
                user.name
            } else {
                format!("{}@{}", user.name, user.instance)
            },
            created_at: user.published,
            note: user.bio.clone().unwrap_or_default(),
            followers_count: followers_count.try_into().unwrap(),
            following_count: following_count.try_into().unwrap(),

            source: match with_source {
                true => Some(AccountSource {
                    sensitive: false,
                    note: user.bio.unwrap_or_default(),
                    fields: vec![],
                    privacy: DbVisibility::Public,
                    language: "en".to_string(),
                    follow_requests_count: follow_requests_count.try_into().unwrap(),
                }),
                false => None,
            },

            header: "https://http.cat/images/404.jpg".to_string(), // TODO: Media
            header_static: "https://http.cat/images/404.jpg".to_string(),
            avatar: "https://http.cat/images/404.jpg".to_string(),
            avatar_static: "https://http.cat/images/404.jpg".to_string(),

            is_cat: user.is_cat,
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
