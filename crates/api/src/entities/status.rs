use std::sync::Arc;

use db::{
    common::timelines::TimelineEntry,
    models::{post::PostRelationship, Post, PostBoost, User},
    types::{DbId, DbVisibility},
};
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

impl From<PostRelationship> for StatusRelationship {
    fn from(relationship: PostRelationship) -> Self {
        StatusRelationship::from(&relationship)
    }
}

impl From<&PostRelationship> for StatusRelationship {
    fn from(relationship: &PostRelationship) -> Self {
        StatusRelationship {
            favourited: relationship.liked,
            reblogged: relationship.boosted,
            muted: false,
            bookmarked: relationship.bookmarked,
            pinned: false,
        }
    }
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
        let stats = post.stats(&state.db_pool).await?;
        let in_reply = match post.in_reply {
            Some(ref post_id) => Post::by_id(post_id, &state.db_pool).await?,
            None => None,
        };

        let mentions = post.mentioned_users(&state.db_pool).await?;
        let account = Account::build(post.author(&state.db_pool).await?, state, false).await?;
        let relationship = if let Some(user_id) = user_id {
            Some(post.relationship(&user_id, &state.db_pool).await?.into())
        } else {
            None
        };

        Ok(Self::raw_build(
            post,
            account,
            stats.boosts_count,
            stats.likes_count,
            mentions,
            in_reply,
            relationship,
        ))
    }

    pub async fn build_from_boost(
        boost: PostBoost,
        original_post: Option<Post>,
        user_id: Option<&DbId>,
        state: &Arc<AppState>,
    ) -> anyhow::Result<Self> {
        let status = Self::build(
            if let Some(original_post) = original_post {
                original_post
            } else {
                boost.post(&state.db_pool).await?
            },
            user_id,
            state,
        )
        .await?;
        let author = Account::build(boost.author(&state.db_pool).await?, state, false).await?;

        Ok(Self::raw_boost_build(boost, status, author))
    }

    pub async fn build_from_vec(
        posts: Vec<Post>,
        user_id: Option<&DbId>,
        state: &Arc<AppState>,
    ) -> anyhow::Result<Vec<Self>> {
        let accounts = Account::build_from_vec(
            User::by_ids(
                posts
                    .iter()
                    .map(|post| &post.author)
                    .collect(),
                &state.db_pool
            )
            .await?
            .into_iter()
            .map(|user| user.expect("complete deletion of a user is not possible; its presence is checked before creating a post"))
            .collect::<Vec<User>>(),
            state,
        )
        .await?;

        let stats =
            Post::stats_by_vec(posts.iter().map(|post| &post.id).collect(), &state.db_pool).await?;

        let in_replies = Post::by_ids(
            posts
                .iter()
                .filter_map(|post| post.in_reply.as_ref())
                .collect(),
            &state.db_pool,
        )
        .await?
        .into_iter()
        .map(|post| post.expect("complete deletion of a post is not possible"))
        .collect::<Vec<Post>>();
        let mut in_replies_iter = in_replies.into_iter();
        let in_replies: Vec<Option<Post>> = posts
            .iter()
            .map(|post| post.in_reply.as_ref().and_then(|_| in_replies_iter.next()))
            .collect();

        let mentions: Vec<Vec<User>> = join_all(
            posts
                .iter()
                .map(|post| async move { post.mentioned_users(&state.db_pool).await }),
        )
        .await
        .into_iter()
        .collect::<anyhow::Result<Vec<_>>>()?;

        let relationships = if let Some(user_id) = user_id {
            Some(
                Post::relationships(
                    posts.iter().map(|post| &post.id).collect(),
                    user_id,
                    &state.db_pool,
                )
                .await?,
            )
        } else {
            None
        };

        Ok(posts
            .into_iter()
            .zip(accounts)
            .zip(in_replies)
            .zip(mentions)
            .map(|(((post, account), in_reply), mentions)| {
                let stats = stats
                    .iter()
                    .find(|stats| stats.post_id == post.id)
                    .expect("each post must be in the result of the request");
                let relationship = relationships.as_ref().map(|relationships| {
                    relationships
                        .iter()
                        .find(|relationship| relationship.post_id == post.id)
                        .expect("each post must be in the result of the request")
                        .into()
                });
                Self::raw_build(
                    post,
                    account,
                    stats.boosts_count,
                    stats.likes_count,
                    mentions,
                    in_reply,
                    relationship,
                )
            })
            .collect())
    }

    pub async fn build_timeline(
        entries: Vec<TimelineEntry>,
        user_id: Option<&DbId>,
        state: &Arc<AppState>,
    ) -> anyhow::Result<Vec<Self>> {
        let post_accounts = Account::build_from_vec(
            User::by_ids(
                entries
                    .iter()
                    .map(|entry| match entry {
                        TimelineEntry::Post(post) | TimelineEntry::Boost(_, post) => &post.author,
                    })
                    .collect(),
                &state.db_pool
            )
            .await?
            .into_iter()
            .map(|user| user.expect("complete deletion of a user is not possible; its presence is checked before creating a post"))
            .collect::<Vec<User>>(),
            state,
        )
        .await?;

        let boost_accounts = Account::build_from_vec(
            User::by_ids(
                entries
                    .iter()
                    .filter_map(|entry| match entry {
                        TimelineEntry::Post(_) => None,
                        TimelineEntry::Boost(boost, _) => Some(&boost.actor_id),
                    })
                    .collect(),
                &state.db_pool
            )
            .await?
            .into_iter()
            .map(|user| user.expect("complete deletion of a user is not possible; its presence is checked before creating a boost"))
            .collect::<Vec<User>>(),
            state,
        )
        .await?;
        let mut boost_accounts_iter = boost_accounts.into_iter();
        let boost_accounts: Vec<Option<Account>> = entries
            .iter()
            .map(|entry| match entry {
                TimelineEntry::Post(_) => None,
                TimelineEntry::Boost(_, _) => boost_accounts_iter.next(),
            })
            .collect();

        let stats = Post::stats_by_vec(
            entries
                .iter()
                .map(|entry| match entry {
                    TimelineEntry::Post(post) | TimelineEntry::Boost(_, post) => &post.id,
                })
                .collect(),
            &state.db_pool,
        )
        .await?;

        let in_replies = Post::by_ids(
            entries
                .iter()
                .filter_map(|entry| match entry {
                    TimelineEntry::Post(post) | TimelineEntry::Boost(_, post) => {
                        post.in_reply.as_ref()
                    },
                })
                .collect(),
            &state.db_pool,
        )
        .await?
        .into_iter()
        .map(|post| post.expect("complete deletion of a post is not possible"))
        .collect::<Vec<Post>>();
        let mut in_replies_iter = in_replies.into_iter();
        let in_replies: Vec<Option<Post>> = entries
            .iter()
            .map(|entry| match entry {
                TimelineEntry::Post(post) | TimelineEntry::Boost(_, post) => {
                    post.in_reply.as_ref().and_then(|_| in_replies_iter.next())
                },
            })
            .collect();

        let mentions: Vec<Vec<User>> = join_all(entries.iter().map(|entry| async move {
            match entry {
                TimelineEntry::Post(post) | TimelineEntry::Boost(_, post) => {
                    post.mentioned_users(&state.db_pool).await
                },
            }
        }))
        .await
        .into_iter()
        .collect::<anyhow::Result<Vec<_>>>()?;

        let relationships = if let Some(user_id) = user_id {
            Some(
                Post::relationships(
                    entries
                        .iter()
                        .map(|entry| match entry {
                            TimelineEntry::Post(post) | TimelineEntry::Boost(_, post) => &post.id,
                        })
                        .collect(),
                    user_id,
                    &state.db_pool,
                )
                .await?,
            )
        } else {
            None
        };

        Ok(entries
            .into_iter()
            .zip(post_accounts)
            .zip(boost_accounts)
            .zip(in_replies)
            .zip(mentions)
            .map(
                |((((entry, post_account), boost_account), in_reply), mentions)| {
                    let stats = stats
                        .iter()
                        .find(|stats| match &entry {
                            TimelineEntry::Post(post) | TimelineEntry::Boost(_, post) => {
                                stats.post_id == post.id
                            },
                        })
                        .expect("each post must be in the result of the request");
                    let relationship = relationships.as_ref().map(|relationships| {
                        relationships
                            .iter()
                            .find(|relationship| match &entry {
                                TimelineEntry::Post(post) | TimelineEntry::Boost(_, post) => {
                                    relationship.post_id == post.id
                                },
                            })
                            .expect("each post must be in the result of the request")
                            .into()
                    });
                    match entry {
                        TimelineEntry::Post(post) => Self::raw_build(
                            post,
                            post_account,
                            stats.boosts_count,
                            stats.likes_count,
                            mentions,
                            in_reply,
                            relationship,
                        ),
                        TimelineEntry::Boost(boost, post) => Self::raw_boost_build(
                            boost,
                            Self::raw_build(
                                post,
                                post_account,
                                stats.boosts_count,
                                stats.likes_count,
                                mentions,
                                in_reply,
                                relationship,
                            ),
                            boost_account.expect("must be here"),
                        ),
                    }
                },
            )
            .collect())
    }

    fn raw_boost_build(boost: PostBoost, status: Status, author: Account) -> Self {
        Status {
            id: boost.id.to_string(),
            uri: boost.ap_id.to_string(),
            url: boost.ap_id.to_string(),
            created_at: boost.published.to_string(),
            account: author,
            visibility: boost.visibility,
            reblog: Some(Box::new(status.clone())),
            ..status
        }
    }

    fn raw_build(
        post: Post,
        author: Account,
        reblogs_count: i64,
        favourites_count: i64,
        mentions: Vec<User>,
        in_reply: Option<Post>,
        relationship: Option<StatusRelationship>,
    ) -> Self {
        Self {
            id: post.id.to_string(),
            uri: post.ap_id.to_string(),
            created_at: post.published.to_string(),
            account: author,
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
            reblogs_count: reblogs_count
                .try_into()
                .expect("Nice, my post has 4294967296 boosts!"),
            favourites_count: favourites_count
                .try_into()
                .expect("Nice, my post has 4294967296 likes!"),
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
        }
    }
}
