use std::sync::Arc;

use activitypub_federation::{config::Data, traits::ActivityHandler};
use db::models::ReceivedActivity;
use serde::{Deserialize, Serialize};
use url::Url;
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
pub enum UserInbox {
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

/**
 * This ensures that the same activity does not get processed more than once.
 */
pub async fn insert_received_activity(
    ap_id: &Url,
    data: &Data<Arc<AppState>>,
) -> anyhow::Result<()> {
    ReceivedActivity::create(ap_id.as_str(), &data.db_pool).await
}
