use serde::Serialize;

// TODO: Fully implement https://docs.joinmastodon.org/entities/Relationship/
#[derive(Serialize, Debug)]
pub struct Relationship {
    pub id: String,
    pub following: bool,
    pub followed_by: bool,
    pub requested: bool,
    pub note: String,
}
