use std::sync::Arc;

use activitypub_federation::{
    config::Data, fetch::object_id::ObjectId, kinds::activity::UndoType,
    protocol::helpers::deserialize_skip_error, traits::ActivityHandler,
};
use async_trait::async_trait;
use db::models::{user_follow_request::UserFollowRequest, user_follower::UserFollower};
use serde::{Deserialize, Serialize};
use url::Url;
use web::AppState;

use crate::{
    activities::{follow::Follow, is_duplicate},
    common::notifications,
    objects::user::ApUser,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UndoFollow {
    pub actor: ObjectId<ApUser>,
    #[serde(deserialize_with = "deserialize_skip_error", default)]
    pub to: Option<[ObjectId<ApUser>; 1]>,
    pub object: Follow,
    #[serde(rename = "type")]
    pub kind: UndoType,
    pub id: Url,
}

#[async_trait]
impl ActivityHandler for UndoFollow {
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
        let actor_follow = self.object.actor.dereference(data).await?;

        if actor_undo.id != actor_follow.id {
            return Err(anyhow::anyhow!("Invalid UndoFollow activity..."));
        }

        self.object.verify(data).await?;
        Ok(())
    }

    async fn receive(self, data: &Data<Self::DataType>) -> Result<(), Self::Error> {
        if is_duplicate(&self.id, data).await? {
            return Ok(());
        }

        let actor = self.actor.dereference(data).await?;
        let followed = self.object.object.dereference(data).await?;

        if UserFollowRequest::delete(&actor, &followed, self.object.id.to_string(), &data.db_pool)
            .await?
        {
            notifications::process_follow_request(&actor, &followed, true, &data.db_pool).await?;
        } else {
            UserFollower::delete(&actor, &followed, self.object.id.to_string(), &data.db_pool)
                .await?;
            notifications::process_follow(&actor, &followed, true, &data.db_pool).await?;
        }

        Ok(())
    }
}
