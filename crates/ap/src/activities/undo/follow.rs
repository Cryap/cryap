use std::sync::Arc;

use activitypub_federation::{
    activity_queue::queue_activity,
    config::Data,
    fetch::object_id::ObjectId,
    kinds::activity::{FollowType, UndoType},
    protocol::helpers::deserialize_skip_error,
    traits::{ActivityHandler, Actor},
};
use async_trait::async_trait;
use db::models::{user_follow_request::UserFollowRequest, user_follower::UserFollower};
use serde::{Deserialize, Serialize};
use url::Url;
use web::AppState;

use crate::{
    activities::{follow::Follow, generate_undo_activity_id, is_duplicate},
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

impl UndoFollow {
    pub async fn send(
        follow_id: Url,
        actor: &ApUser,
        object: &ApUser,
        data: &Data<Arc<AppState>>,
    ) -> anyhow::Result<Url> {
        let id = generate_undo_activity_id(&actor.ap_id, FollowType::Follow)?;
        let activity = UndoFollow {
            actor: actor.id().into(),
            to: Some([ObjectId::<ApUser>::from(object.id())]),
            object: Follow::new(follow_id, actor, object),
            kind: Default::default(),
            id: id.clone(),
        };

        let inboxes = vec![object.shared_inbox_or_inbox()];
        queue_activity(&activity, actor, inboxes, data).await?;

        Ok(id)
    }
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

        if UserFollowRequest::delete(
            &actor,
            &followed,
            Some(self.object.id.to_string()),
            &data.db_pool,
        )
        .await?
        {
            notifications::process_follow_request(&actor, &followed, true, &data.db_pool).await?;
        } else {
            UserFollower::delete(
                &actor,
                &followed,
                Some(self.object.id.to_string()),
                &data.db_pool,
            )
            .await?;
            notifications::process_follow(&actor, &followed, true, &data.db_pool).await?;
        }

        Ok(())
    }
}
