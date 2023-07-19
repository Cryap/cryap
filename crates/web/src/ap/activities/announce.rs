use std::sync::Arc;

use activitypub_federation::kinds::activity::AnnounceType;
use activitypub_federation::{config::Data, fetch::object_id::ObjectId, traits::ActivityHandler};
use async_trait::async_trait;
use db::models::interactions::PostBoost;
use db::{schema::post_boost, schema::post_boost::dsl};
use diesel::insert_into;
use diesel_async::RunQueryDsl;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::ap::objects::note::ApNote;
use crate::{ap::objects::user::ApUser, AppState};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Announce {
    pub(crate) actor: ObjectId<ApUser>,
    pub(crate) object: ObjectId<ApNote>,
    #[serde(rename = "type")]
    pub(crate) kind: AnnounceType,
    pub(crate) id: Url,
}

#[async_trait]
impl ActivityHandler for Announce {
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
        let mut conn = data.db_pool.get().await?;

        let actor = self.actor.dereference(data).await?;
        let post = self.object.dereference(data).await?;

        insert_into(dsl::post_boost)
            .values(vec![PostBoost {
                actor_id: actor.id.clone(),
                post_id: post.id.clone(),
                ap_id: self.id.to_string(),
            }])
            .on_conflict((post_boost::actor_id, post_boost::post_id))
            .do_nothing()
            .execute(&mut conn)
            .await?;

        Ok(())
    }
}
