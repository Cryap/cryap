use std::sync::Arc;

use activitypub_federation::{
    activity_queue::queue_activity,
    config::Data,
    fetch::object_id::ObjectId,
    kinds::activity::{LikeType, UndoType},
    traits::{ActivityHandler, Actor},
};
use async_trait::async_trait;
use db::models::PostLike;
use serde::{Deserialize, Serialize};
use url::Url;
use web::AppState;

use crate::{
    activities::{generate_undo_activity_id, is_duplicate, like::Like},
    common::notifications,
    objects::{note::ApNote, user::ApUser},
};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UndoLike {
    pub actor: ObjectId<ApUser>,
    pub object: Like,
    #[serde(rename = "type")]
    pub kind: UndoType,
    pub id: Url,
}

impl UndoLike {
    pub async fn send(
        like_id: Url,
        actor: &ApUser,
        author: &ApUser,
        note: &ApNote,
        data: &Data<Arc<AppState>>,
    ) -> anyhow::Result<Url> {
        let id = generate_undo_activity_id(&actor.ap_id, LikeType::Like)?;
        let activity = UndoLike {
            actor: actor.id().into(),
            object: Like::new(like_id, actor, note),
            kind: Default::default(),
            id: id.clone(),
        };

        let inboxes = vec![author.shared_inbox_or_inbox()];
        queue_activity(&activity, actor, inboxes, data).await?;

        Ok(id)
    }
}

#[async_trait]
impl ActivityHandler for UndoLike {
    type DataType = Arc<AppState>;
    type Error = anyhow::Error;

    fn id(&self) -> &Url {
        &self.id
    }

    fn actor(&self) -> &Url {
        self.actor.inner()
    }

    async fn verify(&self, data: &Data<Self::DataType>) -> Result<(), Self::Error> {
        let actor_undo = self.actor.dereference(data).await?;
        let actor_like = self.object.actor.dereference(data).await?;

        if actor_undo.id != actor_like.id {
            return Err(anyhow::anyhow!("Invalid Undo activity..."));
        }

        self.object.verify(data).await?;
        Ok(())
    }

    async fn receive(self, data: &Data<Self::DataType>) -> Result<(), Self::Error> {
        if is_duplicate(&self.id, data).await? {
            return Ok(());
        }

        let actor = self.actor.dereference(data).await?;
        let post = self.object.object.dereference(data).await?;

        if PostLike::delete(
            Some(self.object.id.to_string()),
            &post,
            &actor,
            &data.db_pool,
        )
        .await?
        {
            notifications::process_like(
                &post,
                &actor,
                &post.author(&data.db_pool).await?,
                true,
                &data.db_pool,
            )
            .await?;
        }

        Ok(())
    }
}
