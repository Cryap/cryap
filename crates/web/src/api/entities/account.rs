use chrono::NaiveDateTime;
use db::models::User;
use serde::Serialize;

// TODO: Fully implement https://docs.joinmastodon.org/entities/Account/
#[derive(Serialize, Debug)]
pub struct Account {
    pub id: String,
    pub url: String,
    pub username: String,
    pub acct: String,
    pub display_name: String,
    pub created_at: NaiveDateTime,
    pub note: String,
}

impl Account {
    pub fn new(user: User) -> Self {
        Self {
            id: user.id.to_string(),
            url: user.ap_id, // TODO: Discuss
            username: user.name.clone(),
            display_name: user.display_name.unwrap_or(user.name.clone()),
            acct: if user.local {
                user.name
            } else {
                format!("{}@{}", user.name, user.instance)
            },
            created_at: user.published,
            note: user.bio.unwrap_or(String::from("")),
        }
    }
}
