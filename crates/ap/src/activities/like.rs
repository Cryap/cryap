use std::sync::Arc;

use activitypub_federation::{
    config::Data, fetch::object_id::ObjectId, kinds::activity::LikeType, traits::ActivityHandler,
};
use async_trait::async_trait;
use db::models::PostLike;
use serde::{Deserialize, Serialize};
use url::Url;
use web::AppState;

use crate::{
    activities::is_duplicate,
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

        if PostLike::create(self.id.to_string(), &post, &actor, &data.db_pool).await? {
            notifications::process_like(&post, &actor, false, &data.db_pool).await?;
        }

        Ok(())
    }
}
