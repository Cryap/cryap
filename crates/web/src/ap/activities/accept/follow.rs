use std::sync::Arc;

use activitypub_federation::kinds::activity::AcceptType;
use activitypub_federation::protocol::helpers::deserialize_skip_error;
use activitypub_federation::{config::Data, fetch::object_id::ObjectId, traits::ActivityHandler};
use async_trait::async_trait;
use db::schema;
use db::schema::user_follow_requests::dsl::user_follow_requests;
use db::schema::user_followers::dsl;
use db::{models::UserFollowersInsert, schema::user_followers};
use diesel::ExpressionMethods;
use diesel::QueryDsl;
use diesel::{delete, insert_into};
use diesel_async::RunQueryDsl;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::ap::activities::follow::Follow;
use crate::{ap::objects::user::ApUser, AppState};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AcceptFollow {
    pub(crate) actor: ObjectId<ApUser>,
    #[serde(deserialize_with = "deserialize_skip_error", default)]
    pub(crate) to: Option<[ObjectId<ApUser>; 1]>,
    pub(crate) object: Follow,
    #[serde(rename = "type")]
    pub(crate) kind: AcceptType,
    pub(crate) id: Url,
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

        Ok(())
    }

    async fn receive(self, data: &Data<Self::DataType>) -> Result<(), Self::Error> {
        let mut conn = data.db_pool.get().await?;

        let actor = self.actor.dereference(data).await?;
        let followed = self.object.actor.dereference(data).await?;

        let _ = delete(
            user_follow_requests
                .filter(schema::user_follow_requests::actor_id.eq(followed.id.clone()))
                .filter(schema::user_follow_requests::follower_id.eq(actor.id.clone())),
        )
        .execute(&mut conn)
        .await;

        insert_into(dsl::user_followers)
            .values(vec![UserFollowersInsert {
                actor_id: followed.id.clone(),
                follower_id: actor.id.clone(),
                ap_id: Some(self.object.id.to_string()),
            }])
            .on_conflict((user_followers::actor_id, user_followers::follower_id))
            .do_nothing()
            .execute(&mut conn)
            .await?;

        Ok(())
    }
}
