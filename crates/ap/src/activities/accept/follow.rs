use std::sync::Arc;

use activitypub_federation::{
    config::Data, fetch::object_id::ObjectId, kinds::activity::AcceptType,
    protocol::helpers::deserialize_skip_error, traits::ActivityHandler,
};
use async_trait::async_trait;
use db::{
    models::UserFollowersInsert,
    schema,
    schema::{
        user_follow_requests::dsl::user_follow_requests, user_followers, user_followers::dsl,
    },
};
use diesel::{delete, insert_into, ExpressionMethods, QueryDsl};
use diesel_async::RunQueryDsl;
use serde::{Deserialize, Serialize};
use url::Url;
use web::AppState;

use crate::{activities::follow::Follow, objects::user::ApUser};

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
