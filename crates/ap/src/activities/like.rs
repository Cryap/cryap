use std::sync::Arc;

use activitypub_federation::{
    activity_queue::queue_activity,
    config::Data,
    fetch::object_id::ObjectId,
    kinds::activity::LikeType,
    traits::{ActivityHandler, Actor},
};
use async_trait::async_trait;
use db::models::PostLike;
use serde::{Deserialize, Serialize};
use url::Url;
use web::AppState;

use crate::{
    activities::{generate_activity_id, is_duplicate},
    common::notifications,
    objects::{note::ApNote, user::ApUser},
};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Like {
    pub actor: ObjectId<ApUser>,
    pub object: ObjectId<ApNote>,
    #[serde(rename = "type")]
    pub kind: LikeType,
    pub id: Url,
}

impl Like {
    pub(crate) fn new(id: Url, actor: &ApUser, note: &ApNote) -> Like {
        Like {
            actor: actor.id().into(),
            object: note.id().into(),
            kind: Default::default(),
            id,
        }
    }

    pub async fn send(
        actor: &ApUser,
        author: &ApUser,
        note: &ApNote,
        data: &Data<Arc<AppState>>,
    ) -> anyhow::Result<Url> {
        let id = generate_activity_id(&actor.ap_id, LikeType::Like)?;
        let activity = Like::new(id.clone(), actor, note);

        let inboxes = vec![author.shared_inbox_or_inbox()];
        queue_activity(&activity, actor, inboxes, data).await?;

        Ok(id)
    }
}

#[async_trait]
impl ActivityHandler for Like {
    type DataType = Arc<AppState>;
    type Error = anyhow::Error;

    fn id(&self) -> &Url {
        &self.id
    }

    fn actor(&self) -> &Url {
        self.actor.inner()
    }

    async fn verify(&self, _data: &Data<Self::DataType>) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn receive(self, data: &Data<Self::DataType>) -> Result<(), Self::Error> {
        if is_duplicate(&self.id, data).await? {
            return Ok(());
        }

        let actor = self.actor.dereference(data).await?;
        let post = self.object.dereference(data).await?;

        if PostLike::create(Some(self.id.to_string()), &post, &actor, &data.db_pool).await? {
            notifications::process_like(
                &post,
                &actor,
                &post.author(&data.db_pool).await?,
                false,
                &data.db_pool,
            )
            .await?;
        }

        Ok(())
    }
}
