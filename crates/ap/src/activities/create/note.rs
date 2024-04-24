use std::sync::Arc;

use activitypub_federation::{
    activity_queue::queue_activity,
    config::Data,
    fetch::object_id::ObjectId,
    kinds::activity::CreateType,
    traits::{ActivityHandler, Object},
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use url::Url;
use web::AppState;

use crate::{
    activities::is_duplicate,
    common::notifications,
    objects::{
        note::{ApNote, Note},
        user::ApUser,
    },
};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateNote {
    pub(crate) actor: ObjectId<ApUser>,
    pub(crate) object: Note,
    #[serde(rename = "type")]
    pub(crate) kind: CreateType,
    pub(crate) id: Url,
}

impl CreateNote {
    pub async fn send(
        note: ApNote,
        actor: &ApUser,
        mentions: &Vec<ApUser>,
        inboxes: Vec<Url>,
        data: &Data<Arc<AppState>>,
    ) -> anyhow::Result<()> {
        let activity = CreateNote::from(note.into_json_mentions(data, mentions).await?);
        queue_activity(&activity, actor, inboxes, data).await?;
        Ok(())
    }
}

impl From<Note> for CreateNote {
    fn from(note: Note) -> Self {
        let mut id = note.id.clone().into_inner();
        id.set_fragment(Some("create")); // https://cryap/p/id#create
        CreateNote {
            id,
            kind: Default::default(),
            actor: note.attributed_to.clone(),
            object: note.clone(),
        }
    }
}

#[async_trait]
impl ActivityHandler for CreateNote {
    type DataType = Arc<AppState>;
    type Error = anyhow::Error;

    fn id(&self) -> &Url {
        &self.id
    }

    fn actor(&self) -> &Url {
        self.actor.inner()
    }

    async fn verify(&self, data: &Data<Self::DataType>) -> Result<(), Self::Error> {
        let actor_create = self.actor.dereference(data).await?;
        let actor_note = self.object.attributed_to.dereference(data).await?;

        if actor_create.id != actor_note.id {
            return Err(anyhow::anyhow!("Invalid Create activity..."));
        }

        ApNote::verify(&self.object, self.actor.inner(), data).await?;
        Ok(())
    }

    async fn receive(self, data: &Data<Self::DataType>) -> Result<(), Self::Error> {
        if is_duplicate(&self.id, data).await? {
            return Ok(());
        }

        let note = ApNote::from_json(self.object, data).await?;
        notifications::process_post(&note, &data.db_pool).await?;

        Ok(())
    }
}
