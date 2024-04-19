use std::sync::Arc;

use activitypub_federation::{
    config::Data, fetch::object_id::ObjectId, kinds::activity::UndoType, traits::ActivityHandler,
};
use async_trait::async_trait;
use db::{schema, schema::post_like::dsl::post_like};
use diesel::{delete, prelude::*};
use diesel_async::RunQueryDsl;
use serde::{Deserialize, Serialize};
use url::Url;
use web::AppState;

use crate::{
    activities::{insert_received_activity, like::Like},
    common::notifications,
    objects::user::ApUser,
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
        Ok(())
    }

    async fn receive(self, data: &Data<Self::DataType>) -> Result<(), Self::Error> {
        insert_received_activity(&self.id, data).await?;

        let mut conn = data.db_pool.get().await?;

        let actor = self.actor.dereference(data).await?;
        let post = self.object.object.dereference(data).await?;

        let _ = delete(
            post_like
                .filter(schema::post_like::actor_id.eq(actor.id.clone()))
                .filter(schema::post_like::post_id.eq(post.id.clone())),
        )
        .execute(&mut conn)
        .await;

        notifications::process_like(&post, &actor, true, &data.db_pool).await?;

        Ok(())
    }
}
