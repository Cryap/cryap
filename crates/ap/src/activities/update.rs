use std::sync::Arc;

use activitypub_federation::{
    config::Data,
    fetch::object_id::ObjectId,
    kinds::activity::UpdateType,
    protocol::helpers::deserialize_one_or_many,
    traits::{ActivityHandler, Object},
};
use async_trait::async_trait;
use db::{models::User, types::DbId};
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use url::Url;
use web::AppState;

use crate::{
    activities::is_duplicate,
    objects::user::{ApUser, Person},
    PUBLIC,
};

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Update {
    pub actor: ObjectId<ApUser>,
    pub object: Person,
    #[serde(deserialize_with = "deserialize_one_or_many")]
    pub to: Vec<Url>,
    #[serde(rename = "type")]
    pub kind: UpdateType,
    pub id: Url,
}

impl Update {
    pub async fn build(actor: User, data: &Data<Arc<AppState>>) -> anyhow::Result<Self> {
        let id = Url::parse(&format!(
            "{}/activities/updates/{}",
            actor.ap_id,
            DbId::default()
        ))
        .unwrap();
        Ok(Self {
            id,
            kind: Default::default(),
            to: vec![Url::parse(PUBLIC).unwrap()],
            actor: ObjectId::<ApUser>::from(Url::parse(&actor.ap_id).unwrap()),
            object: ApUser(actor).into_json(data).await?,
        })
    }
}

#[async_trait]
impl ActivityHandler for Update {
    type DataType = Arc<AppState>;
    type Error = anyhow::Error;

    fn id(&self) -> &Url {
        &self.id
    }

    fn actor(&self) -> &Url {
        self.actor.inner()
    }

    async fn verify(&self, _data: &Data<Self::DataType>) -> Result<(), Self::Error> {
        if self.actor != self.object.id {
            return Err(anyhow::anyhow!("Invalid Update activity..."));
        }

        Ok(())
    }

    async fn receive(self, data: &Data<Self::DataType>) -> Result<(), Self::Error> {
        if is_duplicate(&self.id, data).await? {
            return Ok(());
        }

        ApUser::from_json(self.object, data).await?;
        Ok(())
    }
}
