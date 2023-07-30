use serde::Serialize;

// https://docs.joinmastodon.org/entities/Rule/
#[derive(Serialize, Debug)]
pub struct Rule {
    pub id: String,
    pub text: String,
}
