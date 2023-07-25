use db::models::Session;
use serde::Serialize;

// TODO: Fully implement https://docs.joinmastodon.org/entities/Token/
#[derive(Serialize, Debug)]
pub struct Token {
    pub access_token: String,
    pub token_type: String,
    pub created_at: i64,
}

impl Token {
    pub fn new(session: Session) -> Self {
        Self {
            access_token: session.token,
            token_type: String::from("Bearer"),
            created_at: session.published.timestamp(),
        }
    }
}
