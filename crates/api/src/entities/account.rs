use std::sync::Arc;

use chrono::{DateTime, Utc};
use db::{models::User, types::DbVisibility};
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
    pub created_at: DateTime<Utc>,
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
        let stats = user.stats(&state.db_pool).await?;
        let follow_requests_count = if with_source {
            Some(user.follow_requests_count(&state.db_pool).await?)
        } else {
            None
        };

        Ok(Self::raw_build(
            user,
            stats.followers_count,
            stats.following_count,
            follow_requests_count,
        ))
    }

    pub async fn build_from_vec(
        users: Vec<User>,
        state: &Arc<AppState>,
    ) -> anyhow::Result<Vec<Self>> {
        let stats =
            User::stats_by_vec(users.iter().map(|user| &user.id).collect(), &state.db_pool).await?;

        Ok(users
            .into_iter()
            .map(|user| {
                let stats = stats
                    .iter()
                    .find(|stats| stats.user_id == user.id)
                    .expect("Each user must be in the result of the request");
                Self::raw_build(user, stats.followers_count, stats.following_count, None)
            })
            .collect())
    }

    fn raw_build(
        user: User,
        followers_count: i64,
        following_count: i64,
        follow_requests_count: Option<i64>,
    ) -> Self {
        Self {
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

            source: if let Some(follow_requests_count) = follow_requests_count {
                Some(AccountSource {
                    sensitive: false,
                    note: user.bio.unwrap_or_default(),
                    fields: vec![],
                    privacy: DbVisibility::Public,
                    language: "en".to_string(),
                    follow_requests_count: follow_requests_count.try_into().unwrap(),
                })
            } else {
                None
            },

            header: "https://http.cat/images/404.jpg".to_string(), // TODO: Media
            header_static: "https://http.cat/images/404.jpg".to_string(),
            avatar: "https://http.cat/images/404.jpg".to_string(),
            avatar_static: "https://http.cat/images/404.jpg".to_string(),

            is_cat: user.is_cat,
        }
    }
}
