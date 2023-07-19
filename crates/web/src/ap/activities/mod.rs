use activitypub_federation::config::Data;
use activitypub_federation::traits::ActivityHandler;
use serde::{Deserialize, Serialize};
use url::Url;

pub mod accept;
pub mod create;
pub mod follow;
pub mod undo;
pub mod like;
pub mod announce;

#[derive(Deserialize, Serialize, Debug)]
#[serde(untagged)]
#[enum_delegate::implement(ActivityHandler)]
pub enum UserInbox {
    Follow(follow::Follow),
    AcceptFollow(accept::follow::AcceptFollow),
    UndoFollow(undo::follow::UndoFollow),
    CreateNote(create::note::CreateNote),
    Like(like::Like),
    UndoLike(undo::like::UndoLike),
    Announce(announce::Announce),
    UndoAnnounce(undo::announce::UndoAnnounce),
}
