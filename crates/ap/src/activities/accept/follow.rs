use std::sync::Arc;

use activitypub_federation::{
    activity_queue::queue_activity,
    config::Data,
    fetch::object_id::ObjectId,
    kinds::activity::{AcceptType, FollowType},
    protocol::helpers::deserialize_skip_error,
    traits::{ActivityHandler, Actor},
};
use async_trait::async_trait;
use db::models::{user_follow_request::UserFollowRequest, user_follower::UserFollower};
use serde::{Deserialize, Serialize};
use url::Url;
use web::AppState;

use crate::{
    activities::{follow::Follow, generate_accept_activity_id, is_duplicate},
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

impl AcceptFollow {
    pub async fn send(
        follow_id: Url,
        actor: &ApUser,
        object: &ApUser,
        data: &Data<Arc<AppState>>,
    ) -> anyhow::Result<Url> {
        let id = generate_accept_activity_id(&actor.ap_id, FollowType::Follow)?;
        let activity = AcceptFollow {
            actor: actor.id().into(),
            to: Some([ObjectId::<ApUser>::from(object.id())]),
            object: Follow::new(follow_id, object, actor),
            kind: Default::default(),
            id: id.clone(),
        };

        let inboxes = vec![object.shared_inbox_or_inbox()];
        queue_activity(&activity, actor, inboxes, data).await?;

        Ok(id)
    }
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

        if UserFollowRequest::delete(
            &followed,
            &actor,
            Some(self.object.id.to_string()),
            &data.db_pool,
        )
        .await?
        {
            UserFollower::create(
                &followed,
                &actor,
                Some(self.object.id.to_string()),
                &data.db_pool,
            )
            .await?;
        }

        Ok(())
    }
}
