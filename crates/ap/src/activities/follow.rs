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
use db::{
    models::{user_follow_request::UserFollowRequest, user_follower::UserFollower, User},
    types::DbId,
};
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use url::Url;
use web::AppState;

use crate::{
    activities::{accept::follow::AcceptFollow, is_duplicate},
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
    pub fn new(id: Option<DbId>, by: User, to: User) -> Self {
        let id = Url::parse(&format!(
            "{}/activities/follows/{}",
            by.ap_id,
            id.unwrap_or_default()
        ))
        .unwrap(); // TODO: Review
        Self {
            id: id.clone(),
            kind: Default::default(),
            actor: ObjectId::<ApUser>::from(Url::parse(&by.ap_id).unwrap()),
            object: ObjectId::<ApUser>::from(Url::parse(&to.ap_id).unwrap()),
            to: Some([ObjectId::<ApUser>::from(Url::parse(&to.ap_id).unwrap())]),
        }
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
            if UserFollowRequest::create(&actor, &followed, self.id.to_string(), &data.db_pool)
                .await?
            {
                notifications::process_follow_request(&actor, &followed, false, &data.db_pool)
                    .await?;
            }
        } else {
            if UserFollower::create(&actor, &followed, self.id.to_string(), &data.db_pool).await? {
                let activity = AcceptFollow {
                    actor: ObjectId::<ApUser>::from(Url::parse(&followed.ap_id)?),
                    object: self,
                    kind: Default::default(),
                    to: Some([ObjectId::<ApUser>::from(Url::parse(&actor.ap_id)?)]),
                    id: Url::parse(&format!(
                        "{}/activities/accept/follows/{}",
                        followed.ap_id,
                        DbId::default()
                    ))?,
                };

                let inboxes = vec![actor.shared_inbox_or_inbox()];
                queue_activity(&activity, &followed, inboxes, data).await?;

                notifications::process_follow(&actor, &followed, false, &data.db_pool).await?;
            }
        }

        Ok(())
    }
}
