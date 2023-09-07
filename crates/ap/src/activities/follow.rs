use std::sync::Arc;

use activitypub_federation::{
    activity_queue::send_activity,
    config::Data,
    fetch::object_id::ObjectId,
    kinds::activity::FollowType,
    protocol::helpers::deserialize_skip_error,
    traits::{ActivityHandler, Actor},
};
use async_trait::async_trait;
use db::{
    models::{User, UserFollowRequestsInsert, UserFollowersInsert},
    schema::{user_follow_requests, user_followers},
    types::DbId,
};
use diesel::insert_into;
use diesel_async::RunQueryDsl;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use url::Url;
use web::AppState;

use crate::{
    activities::accept::follow::AcceptFollow, common::notifications, objects::user::ApUser,
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
    pub fn new(id: Option<DbId>, by: User, to: User) -> Follow {
        let id = Url::parse(&format!(
            "{}/activities/follows/{}",
            by.ap_id,
            id.unwrap_or_default()
        ))
        .unwrap(); // TODO: Review
        Follow {
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
        let mut conn = data.db_pool.get().await?;

        let actor = self.actor.dereference(data).await?;
        let followed = self.object.dereference(data).await?;

        if followed.manually_approves_followers {
            insert_into(user_follow_requests::dsl::user_follow_requests)
                .values(vec![UserFollowRequestsInsert {
                    actor_id: actor.id.clone(),
                    follower_id: followed.id.clone(),
                    ap_id: Some(self.id.to_string()),
                }])
                .on_conflict((
                    user_follow_requests::actor_id,
                    user_follow_requests::follower_id,
                ))
                .do_nothing()
                .execute(&mut conn)
                .await?;

            notifications::process_follow_request(&actor, &followed, false, &data.db_pool).await?;
        } else {
            insert_into(user_followers::dsl::user_followers)
                .values(vec![UserFollowersInsert {
                    actor_id: actor.id.clone(),
                    follower_id: followed.id.clone(),
                    ap_id: Some(self.id.to_string()),
                }])
                .on_conflict((user_followers::actor_id, user_followers::follower_id))
                .do_nothing()
                .execute(&mut conn)
                .await?;

            let activity = AcceptFollow {
                actor: ObjectId::<ApUser>::from(Url::parse(&followed.ap_id)?),
                object: self,
                kind: Default::default(),
                to: Some([ObjectId::<ApUser>::from(Url::parse(&actor.ap_id)?)]),
                id: Url::parse(&format!(
                    "{}/activities/accept/follows/{}",
                    followed.ap_id,
                    DbId::default().to_string()
                ))?,
            };

            let inboxes = vec![actor.shared_inbox_or_inbox()];
            send_activity(activity, &followed, inboxes, data).await?;

            notifications::process_follow(&actor, &followed, false, &data.db_pool).await?;
        }

        Ok(())
    }
}
