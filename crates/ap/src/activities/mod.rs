use std::sync::Arc;

use activitypub_federation::{config::Data, traits::ActivityHandler};
use db::{models::ReceivedActivity, types::DbId};
use diesel::result::{DatabaseErrorKind, Error::DatabaseError};
use serde::{Deserialize, Serialize};
use url::{ParseError, Url};
use web::AppState;

pub mod accept;
pub mod announce;
pub mod create;
pub mod follow;
pub mod like;
pub mod reject;
pub mod undo;
pub mod update;

#[deny(clippy::large_enum_variant)]
#[derive(Deserialize, Serialize, Debug)]
#[serde(untagged)]
#[enum_delegate::implement(ActivityHandler)]
pub enum Inbox {
    Follow(follow::Follow),
    AcceptFollow(accept::follow::AcceptFollow),
    UndoFollow(undo::follow::UndoFollow),
    RejectFollow(reject::follow::RejectFollow),
    CreateNote(create::note::CreateNote),
    Like(like::Like),
    UndoLike(undo::like::UndoLike),
    Announce(announce::Announce),
    UndoAnnounce(undo::announce::UndoAnnounce),
    Update(update::Update),
}

pub fn generate_activity_id<T>(ap_id: &str, kind: T) -> Result<Url, ParseError>
where
    T: ToString,
{
    let id = format!(
        "{}/activities/{}/{}",
        ap_id,
        kind.to_string().to_lowercase(),
        DbId::default()
    );
    Url::parse(&id)
}

pub fn generate_undo_activity_id<T>(ap_id: &str, kind: T) -> Result<Url, ParseError>
where
    T: ToString,
{
    let id = format!(
        "{}/activities/undo/{}/{}",
        ap_id,
        kind.to_string().to_lowercase(),
        DbId::default()
    );
    Url::parse(&id)
}

pub fn generate_accept_activity_id<T>(ap_id: &str, kind: T) -> Result<Url, ParseError>
where
    T: ToString,
{
    let id = format!(
        "{}/activities/accept/{}/{}",
        ap_id,
        kind.to_string().to_lowercase(),
        DbId::default()
    );
    Url::parse(&id)
}

pub fn generate_reject_activity_id<T>(ap_id: &str, kind: T) -> Result<Url, ParseError>
where
    T: ToString,
{
    let id = format!(
        "{}/activities/reject/{}/{}",
        ap_id,
        kind.to_string().to_lowercase(),
        DbId::default()
    );
    Url::parse(&id)
}

pub async fn is_duplicate(ap_id: &Url, data: &Data<Arc<AppState>>) -> anyhow::Result<bool> {
    match ReceivedActivity::create(ap_id.as_str(), &data.db_pool).await {
        Err(DatabaseError(DatabaseErrorKind::UniqueViolation, _)) => Ok(true),
        Err(error) => Err(error.into()),
        Ok(()) => Ok(false),
    }
}
