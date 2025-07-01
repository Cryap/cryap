use chrono::{DateTime, NaiveDate, Utc};
use db::{models::User, types::DbVisibility};
use serde::Serialize;

#[derive(Clone, Serialize, Debug, PartialEq)]
pub struct AccountSource {
    note: String,
    fields: Vec<()>,
    privacy: DbVisibility,
    sensitive: bool,
    language: String,
    follow_requests_count: u32,
}

// TODO: Fully implement https://docs.joinmastodon.org/entities/Account/
#[derive(Clone, Serialize, Debug, PartialEq)]
pub struct Account {
    pub id: String,
    pub url: String,
    pub uri: String,
    pub username: String,
    pub acct: String,
    pub display_name: String,
    pub locked: bool,
    pub bot: bool,
    pub created_at: DateTime<Utc>,
    pub note: String,
    pub followers_count: u32,
    pub following_count: u32,
    pub statuses_count: u32,
    pub last_status_at: Option<NaiveDate>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<AccountSource>,

    pub header: String,
    pub avatar: String,
    pub header_static: String,
    pub avatar_static: String,

    pub is_cat: bool,
}

impl Account {
    pub fn new(user: User, with_source: bool) -> Self {
        Self {
            id: user.id.to_string(),
            url: user.ap_id.clone(), // TODO: Discuss
            uri: user.ap_id,
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
            followers_count: user.followers_count.try_into().unwrap(),
            following_count: user.following_count.try_into().unwrap(),
            statuses_count: user.posts_count.try_into().unwrap(),
            last_status_at: user
                .last_post_published
                .map(|date_time| date_time.date_naive()),

            source: if with_source {
                Some(AccountSource {
                    sensitive: false,
                    note: user.bio.unwrap_or_default(),
                    fields: vec![],
                    privacy: DbVisibility::Public,
                    language: "en".to_string(),
                    follow_requests_count: user.follow_requests_count.try_into().unwrap(),
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

    pub fn new_from_vec(users: Vec<User>) -> Vec<Self> {
        users
            .into_iter()
            .map(|user| Self::new(user, false))
            .collect()
    }
}
