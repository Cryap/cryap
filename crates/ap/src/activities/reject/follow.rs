use std::sync::Arc;

use activitypub_federation::{
    config::Data, fetch::object_id::ObjectId, kinds::activity::RejectType,
    protocol::helpers::deserialize_skip_error, traits::ActivityHandler,
};
use async_trait::async_trait;
use db::schema::{self, user_follow_requests::dsl::user_follow_requests, user_followers::dsl};
use diesel::{delete, ExpressionMethods, QueryDsl};
use diesel_async::RunQueryDsl;
use serde::{Deserialize, Serialize};
use url::Url;
use web::AppState;

use crate::{activities::follow::Follow, objects::user::ApUser};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RejectFollow {
    pub actor: ObjectId<ApUser>,
    #[serde(deserialize_with = "deserialize_skip_error", default)]
    pub to: Option<[ObjectId<ApUser>; 1]>,
    pub object: Follow,
    #[serde(rename = "type")]
    pub kind: RejectType,
    pub id: Url,
}

#[async_trait]
impl ActivityHandler for RejectFollow {
    type DataType = Arc<AppState>;
    type Error = anyhow::Error;

    fn id(&self) -> &Url {
        &self.id
    }

    fn actor(&self) -> &Url {
        self.actor.inner()
    }

    async fn verify(&self, data: &Data<Self::DataType>) -> Result<(), Self::Error> {
        let actor_reject = self.actor.dereference(data).await?;
        let object_follow = self.object.object.dereference(data).await?;

        if actor_reject.id != object_follow.id {
            return Err(anyhow::anyhow!("Invalid RejectFollow activity..."));
        }

        Ok(())
    }

    async fn receive(self, data: &Data<Self::DataType>) -> Result<(), Self::Error> {
        let mut conn = data.db_pool.get().await?;

        let actor = self.actor.dereference(data).await?;
        let followed = self.object.object.dereference(data).await?;

        let _ = delete(
            user_follow_requests
                .filter(schema::user_follow_requests::actor_id.eq(followed.id.clone()))
                .filter(schema::user_follow_requests::follower_id.eq(actor.id.clone())),
        )
        .execute(&mut conn)
        .await;

        let _ = delete(
            dsl::user_followers
                .filter(schema::user_followers::actor_id.eq(followed.id.clone()))
                .filter(schema::user_followers::follower_id.eq(actor.id.clone())),
        )
        .execute(&mut conn)
        .await;

        Ok(())
    }
}
