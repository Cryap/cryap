use std::sync::Arc;

use anyhow::anyhow;
use db::{
    models::{Post, User},
    schema::{
        post_boost::{dsl as post_boost_dsl, dsl::post_boost},
        post_like::{dsl as post_like_dsl, dsl::post_like},
        post_mention::{dsl as post_mention_dsl, dsl::post_mention},
        users::dsl::users,
    },
    types::DbVisibility,
};
use diesel::{ExpressionMethods, QueryDsl, SelectableHelper};
use diesel_async::RunQueryDsl;
use serde::Serialize;
use web::AppState;

use super::Account;

#[derive(Serialize, Debug)]
pub struct StatusMention {
    pub id: String,
    pub username: String,
    pub url: String,
    pub acct: String,
}

#[derive(Serialize, Debug)]
pub struct StatusRelationship {
    pub favourited: bool,
    pub reblogged: bool,
    pub muted: bool,
    pub bookmarked: bool,
    pub pinned: bool,
    // filtered
}

// TODO: Fully implement https://docs.joinmastodon.org/entities/Status/
#[derive(Serialize, Debug)]
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
        status: Post,
        _actor: Option<db::models::User>, // TODO
        data: &Arc<AppState>,
    ) -> anyhow::Result<Self> {
        let mut conn = data.db_pool.get().await?;

        match status.visibility {
            DbVisibility::Public | DbVisibility::Unlisted => {}
            _ => return Err(anyhow!("Access forbidden")), // TODO: Access check
        }

        let reblogs_count: i64 = post_boost
            .filter(post_boost_dsl::post_id.eq(status.id.clone()))
            .count()
            .get_result(&mut conn)
            .await?;
        let favourites_count: i64 = post_like
            .filter(post_like_dsl::post_id.eq(status.id.clone()))
            .count()
            .get_result(&mut conn)
            .await?;

        let reblogs_count: u32 = reblogs_count.try_into().unwrap(); // Nice, my post has
                                                                    // 4294967296 boosts!
        let favourites_count: u32 = favourites_count.try_into().unwrap();

        let mentions: Vec<User> = post_mention
            .filter(post_mention_dsl::post_id.eq(status.id.clone()))
            .inner_join(users)
            .select(User::as_select())
            .load(&mut conn)
            .await?;

        let in_reply = match status.in_reply.clone() {
            Some(post_id) => Post::by_id(&post_id, &data.db_pool).await?,
            None => None,
        };

        Ok(Status {
            id: status.id.to_string(),
            uri: status.ap_id.to_string(),
            created_at: status.published.to_string(),
            account: Account::new(
                match User::by_id(&status.author.clone(), &data.db_pool).await? {
                    Some(user) => user,
                    None => unreachable!(),
                },
            ),
            content: status.content.clone(),
            visibility: status.visibility,
            sensitive: status.sensitive,
            spoiler_text: status.content_warning.unwrap_or("".to_string()),
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
            url: status.ap_id.to_string(),
            in_reply_to_id: status.in_reply.map(|id| id.to_string()),
            in_reply_to_account_id: in_reply.map(|status| status.author.to_string()),
            reblog: None,
            pool: None,
            card: None,
            language: None,
            text: status.content, // TODO: remove html tags maybe
            edited_at: None,
            relationship: None,
        })
    }
}
