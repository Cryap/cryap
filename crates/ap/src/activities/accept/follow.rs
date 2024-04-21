use std::sync::Arc;

use activitypub_federation::{
    config::Data, fetch::object_id::ObjectId, kinds::activity::AcceptType,
    protocol::helpers::deserialize_skip_error, traits::ActivityHandler,
};
use async_trait::async_trait;
use db::models::{user_follow_request::UserFollowRequest, user_follower::UserFollower};
use serde::{Deserialize, Serialize};
use url::Url;
use web::AppState;

use crate::{
    activities::{follow::Follow, is_duplicate},
    objects::user::ApUser,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AcceptFollow {
    pub actor: ObjectId<ApUser>,
    #[serde(deserialize_with = "deserialize_skip_error", default)]
    pub to: Option<[ObjectId<ApUser>; 1]>,
    pub object: Follow,
    #[serde(rename = "type")]
    pub kind: AcceptType,
    pub id: Url,
}

#[async_trait]
impl ActivityHandler for AcceptFollow {
    type DataType = Arc<AppState>;
    type Error = anyhow::Error;

    fn id(&self) -> &Url {
        &self.id
    }

    fn actor(&self) -> &Url {
        self.actor.inner()
    }

    async fn verify(&self, data: &Data<Self::DataType>) -> Result<(), Self::Error> {
        let actor_accept = self.actor.dereference(data).await?;
        let object_follow = self.object.object.dereference(data).await?;

        if actor_accept.id != object_follow.id {
            return Err(anyhow::anyhow!("Invalid Accept activity..."));
        }

        self.object.verify(data).await?;
        Ok(())
    }

    async fn receive(self, data: &Data<Self::DataType>) -> Result<(), Self::Error> {
        if is_duplicate(&self.id, data).await? {
            return Ok(());
        }

        let actor = self.actor.dereference(data).await?;
        let followed = self.object.actor.dereference(data).await?;

        if UserFollowRequest::delete(&followed, &actor, self.object.id.to_string(), &data.db_pool)
            .await?
        {
            UserFollower::create(&followed, &actor, self.id.to_string(), &data.db_pool).await?;
        }

        Ok(())
    }
}
