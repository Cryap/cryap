use std::sync::Arc;

use db::{
    common::timelines::TimelineEntry,
    models::{Post, PostBoost, User},
    schema::{
        post_boost::{dsl as post_boost_dsl, dsl::post_boost},
        post_like::{dsl as post_like_dsl, dsl::post_like},
        post_mention::{dsl as post_mention_dsl, dsl::post_mention},
        users::dsl::users,
    },
    types::{DbId, DbVisibility},
};
use diesel::{ExpressionMethods, QueryDsl, SelectableHelper};
use diesel_async::RunQueryDsl;
use futures::future::join_all;
use serde::Serialize;
use web::AppState;

use super::Account;

#[derive(Clone, Serialize, Debug)]
pub struct StatusMention {
    pub id: String,
    pub username: String,
    pub url: String,
    pub acct: String,
}

#[derive(Clone, Serialize, Debug)]
pub struct StatusRelationship {
    pub favourited: bool,
    pub reblogged: bool,
    pub muted: bool,
    pub bookmarked: bool,
    pub pinned: bool,
    // filtered
}

// TODO: Fully implement https://docs.joinmastodon.org/entities/Status/
#[derive(Clone, Serialize, Debug)]
pub struct Status {
    pub id: String,
    pub uri: String,
    pub created_at: String,
    pub account: Account,
    pub content: String,
    pub visibility: DbVisibility,
    pub sensitive: bool,
    pub spoiler_text: String,
    pub mentions: Vec<StatusMention>,
    // tags
    // emojis
    pub reblogs_count: u32,
    pub favourites_count: u32,
    pub replies_count: u32,
    pub url: String,
    pub in_reply_to_id: Option<String>,
    pub in_reply_to_account_id: Option<String>,
    pub reblog: Option<Box<Status>>,
    pub pool: Option<()>,
    pub card: Option<()>,
    pub language: Option<String>,
    pub text: String,
    pub edited_at: Option<String>,

    #[serde(flatten)]
    pub relationship: Option<StatusRelationship>,
}

impl Status {
    pub async fn build(
        post: Post,
        user_id: Option<&DbId>,
        state: &Arc<AppState>,
    ) -> anyhow::Result<Self> {
        let mut conn = state.db_pool.get().await?;

        let reblogs_count: i64 = post_boost
            .filter(post_boost_dsl::post_id.eq(post.id.clone()))
            .count()
            .get_result(&mut conn)
            .await?;
        let favourites_count: i64 = post_like
            .filter(post_like_dsl::post_id.eq(post.id.clone()))
            .count()
            .get_result(&mut conn)
            .await?;

        let reblogs_count: u32 = reblogs_count.try_into().unwrap(); // Nice, my post has
                                                                    // 4294967296 boosts!
        let favourites_count: u32 = favourites_count.try_into().unwrap();

        let mentions: Vec<User> = post_mention
            .filter(post_mention_dsl::post_id.eq(post.id.clone()))
            .inner_join(users)
            .select(User::as_select())
            .load(&mut conn)
            .await?;

        let in_reply = match post.in_reply.clone() {
            Some(post_id) => Post::by_id(&post_id, &state.db_pool).await?,
            None => None,
        };

        let relationship = if let Some(user_id) = user_id {
            Some(StatusRelationship {
                favourited: post.is_liked_by(user_id, &state.db_pool).await?,
                reblogged: post.boost_by(user_id, &state.db_pool).await?.is_some(),
                muted: false,
                bookmarked: post.bookmarked_by(user_id, &state.db_pool).await?,
                pinned: false,
            })
        } else {
            None
        };

        Ok(Status {
            id: post.id.to_string(),
            uri: post.ap_id.to_string(),
            created_at: post.published.to_string(),
            account: Account::build(post.author(&state.db_pool).await?, state, false).await?,
            content: post.content.clone(),
            visibility: post.visibility,
            sensitive: post.sensitive,
            spoiler_text: post.content_warning.unwrap_or("".to_string()),
            mentions: mentions
                .into_iter()
                .map(|user| StatusMention {
                    id: user.id.to_string(),
                    url: user.ap_id.to_string(),
                    acct: if user.local {
                        user.name.clone()
                    } else {
                        format!("{}@{}", user.name.clone(), user.instance)
                    },
                    username: user.name,
                })
                .collect(),
            reblogs_count,
            favourites_count,
            replies_count: 0, // TODO: Find way to efficiently count replies of specific post
            url: post.ap_id.to_string(),
            in_reply_to_id: post.in_reply.map(|id| id.to_string()),
            in_reply_to_account_id: in_reply.map(|post| post.author.to_string()),
            reblog: None,
            pool: None,
            card: None,
            language: None,
            text: post.content, // TODO: remove html tags maybe
            edited_at: None,
            relationship,
        })
    }

    pub async fn build_from_boost(
        boost: PostBoost,
        user_id: Option<&DbId>,
        state: &Arc<AppState>,
    ) -> anyhow::Result<Self> {
        let post = boost.post(&state.db_pool).await?;
        let status = Self::build(post, user_id, state).await?;

        Ok(Status {
            id: boost.id.to_string(),
            uri: boost.ap_id.to_string(),
            url: boost.ap_id.to_string(),
            created_at: boost.published.to_string(),
            account: Account::build(boost.author(&state.db_pool).await?, state, false).await?,
            visibility: boost.visibility,
            reblog: Some(Box::new(status.clone())),
            ..status
        })
    }

    pub async fn build_from_timeline_entry(
        timeline_entry: TimelineEntry,
        user_id: Option<&DbId>,
        state: &Arc<AppState>,
    ) -> anyhow::Result<Self> {
        match timeline_entry {
            TimelineEntry::Post(post) => Self::build(post, user_id, state).await,
            TimelineEntry::Boost(boost, post) => {
                let status = Self::build(post, user_id, state).await?;
                Ok(Status {
                    id: boost.id.to_string(),
                    uri: boost.ap_id.to_string(),
                    url: boost.ap_id.to_string(),
                    created_at: boost.published.to_string(),
                    account: Account::build(boost.author(&state.db_pool).await?, state, false)
                        .await?,
                    visibility: boost.visibility,
                    reblog: Some(Box::new(status.clone())),
                    ..status
                })
            },
        }
    }

    pub async fn build_from_vec(
        posts: Vec<Post>,
        user_id: Option<&DbId>,
        state: &Arc<AppState>,
    ) -> anyhow::Result<Vec<Self>> {
        join_all(
            posts
                .into_iter()
                .map(|post| async { Self::build(post, user_id, state).await }),
        )
        .await
        .into_iter()
        .collect()
    }

    pub async fn build_timeline(
        entries: Vec<TimelineEntry>,
        user_id: Option<&DbId>,
        state: &Arc<AppState>,
    ) -> anyhow::Result<Vec<Self>> {
        join_all(
            entries.into_iter().map(|entry| async {
                Self::build_from_timeline_entry(entry, user_id, state).await
            }),
        )
        .await
        .into_iter()
        .collect()
    }
}
