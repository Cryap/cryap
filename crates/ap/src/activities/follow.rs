use std::sync::Arc;

use activitypub_federation::{
    activity_queue::queue_activity,
    config::Data,
    fetch::object_id::ObjectId,
    kinds::activity::FollowType,
    protocol::helpers::deserialize_skip_error,
    traits::{ActivityHandler, Actor},
};
use async_trait::async_trait;
use db::models::{user_follow_request::UserFollowRequest, user_follower::UserFollower};
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use url::Url;
use web::AppState;

use crate::{
    activities::{accept::follow::AcceptFollow, generate_activity_id, is_duplicate},
    common::notifications,
    objects::user::ApUser,
};

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Follow {
    pub actor: ObjectId<ApUser>,
    pub object: ObjectId<ApUser>,
    #[serde(deserialize_with = "deserialize_skip_error", default)]
    pub to: Option<[ObjectId<ApUser>; 1]>,
    #[serde(rename = "type")]
    pub kind: FollowType,
    pub id: Url,
}

impl Follow {
    pub(crate) fn new(id: Url, actor: &ApUser, object: &ApUser) -> Follow {
        Follow {
            actor: actor.id().into(),
            object: object.id().into(),
            to: Some([ObjectId::<ApUser>::from(object.id())]),
            kind: Default::default(),
            id,
        }
    }

    pub async fn send(
        actor: &ApUser,
        object: &ApUser,
        data: &Data<Arc<AppState>>,
    ) -> anyhow::Result<Url> {
        let id = generate_activity_id(&actor.ap_id, FollowType::Follow)?;
        let activity = Follow::new(id.clone(), actor, object);

        let inboxes = vec![object.shared_inbox_or_inbox()];
        queue_activity(&activity, actor, inboxes, data).await?;

        Ok(id)
    }
}

#[async_trait]
impl ActivityHandler for Follow {
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
        let followed = self.object.dereference(data).await?;

        if followed.manually_approves_followers {
            if UserFollowRequest::create(
                &actor,
                &followed,
                Some(self.id.to_string()),
                &data.db_pool,
            )
            .await?
            {
                notifications::process_follow_request(&actor, &followed, false, &data.db_pool)
                    .await?;
            }
        } else {
            if UserFollower::create(&actor, &followed, Some(self.id.to_string()), &data.db_pool)
                .await?
            {
                AcceptFollow::send(self.id, &followed, &actor, data).await?;
                notifications::process_follow(&actor, &followed, false, &data.db_pool).await?;
            }
        }

        Ok(())
    }
}
