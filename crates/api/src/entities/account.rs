use std::sync::Arc;

use chrono::{DateTime, Utc};
use db::{models::User, schema::user_follow_requests, types::DbVisibility};
use diesel::{
    sql_query,
    sql_types::{BigInt, Bpchar},
    ExpressionMethods, QueryDsl, QueryableByName,
};
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

#[derive(QueryableByName, Debug)]
struct CountQueryResult {
    #[diesel(sql_type = BigInt)]
    followers_count: i64,
    #[diesel(sql_type = BigInt)]
    following_count: i64,
}

impl Account {
    pub async fn build(
        user: User,
        state: &Arc<AppState>,
        with_source: bool,
    ) -> anyhow::Result<Self> {
        let mut conn = state.db_pool.get().await?;

        // TODO: Rewrite to DSL after Diesel 2.2 release with `case_when`
        let count_query_result: Vec<CountQueryResult> = sql_query(
            "
            SELECT
                SUM(CASE WHEN follower_id = $1 THEN 1 ELSE 0 END) AS followers_count,
                SUM(CASE WHEN actor_id = $1 THEN 1 ELSE 0 END) AS following_count
            FROM user_followers;
            ",
        )
        .bind::<Bpchar, _>(user.id.clone())
        .load::<CountQueryResult>(&mut conn)
        .await?;
        let count_query_result = count_query_result
            .get(0)
            .expect("Result of this query should be not empty");
        let follow_requests_count: i64 = match with_source {
            true => {
                user_follow_requests::table
                    .filter(user_follow_requests::follower_id.eq(&user.id))
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
            followers_count: count_query_result.followers_count.try_into().unwrap(),
            following_count: count_query_result.following_count.try_into().unwrap(),

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
