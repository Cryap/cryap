use std::sync::Arc;

use activitypub_federation::{
    config::Data, fetch::object_id::ObjectId, kinds::activity::UndoType, traits::ActivityHandler,
};
use async_trait::async_trait;
use db::{schema, schema::post_boost::dsl::post_boost};
use diesel::{delete, prelude::*};
use diesel_async::RunQueryDsl;
use serde::{Deserialize, Serialize};
use url::Url;
use web::AppState;

use crate::{
    activities::{announce::Announce, is_duplicate},
    common::notifications,
    objects::user::ApUser,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UndoAnnounce {
    pub(crate) actor: ObjectId<ApUser>,
    pub(crate) object: Announce,
    #[serde(rename = "type")]
    pub(crate) kind: UndoType,
    pub(crate) id: Url,
}

#[async_trait]
impl ActivityHandler for UndoAnnounce {
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

        let mut conn = data.db_pool.get().await?;

        let actor = self.actor.dereference(data).await?;
        let post = self.object.object.dereference(data).await?;

        let _ = delete(
            post_boost
                .filter(schema::post_boost::actor_id.eq(actor.id.clone()))
                .filter(schema::post_boost::post_id.eq(post.id.clone())),
        )
        .execute(&mut conn)
        .await;

        notifications::process_boost(
            &post,
            &actor,
            &post.author(&data.db_pool).await?,
            true,
            &data.db_pool,
        )
        .await?;

        Ok(())
    }
}
