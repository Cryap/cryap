use std::sync::Arc;

use activitypub_federation::{
    config::Data,
    fetch::object_id::ObjectId,
    kinds::activity::CreateType,
    traits::{ActivityHandler, Object},
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{
    ap::objects::{
        note::{ApNote, Note},
        user::ApUser,
    },
    AppState,
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

        Ok(())
    }

    async fn receive(self, data: &Data<Self::DataType>) -> Result<(), Self::Error> {
        ApNote::from_json(self.object, data).await?;

        Ok(())
    }
}
