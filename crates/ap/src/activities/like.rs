use std::sync::Arc;

use activitypub_federation::{
    config::Data, fetch::object_id::ObjectId, kinds::activity::LikeType, traits::ActivityHandler,
};
use async_trait::async_trait;
use chrono::Utc;
use db::{
    models::PostLike,
    schema::{post_like, post_like::dsl},
};
use diesel::insert_into;
use diesel_async::RunQueryDsl;
use serde::{Deserialize, Serialize};
use url::Url;
use web::AppState;

use crate::{
    common::notifications,
    objects::{note::ApNote, user::ApUser},
};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Like {
    pub actor: ObjectId<ApUser>,
    pub object: ObjectId<ApNote>,
    #[serde(rename = "type")]
    pub kind: LikeType,
    pub id: Url,
}

#[async_trait]
impl ActivityHandler for Like {
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
        let post = self.object.dereference(data).await?;

        insert_into(dsl::post_like)
            .values(vec![PostLike {
                actor_id: actor.id.clone(),
                post_id: post.id.clone(),
                ap_id: self.id.to_string(),
                published: Utc::now(),
            }])
            .on_conflict((post_like::actor_id, post_like::post_id))
            .do_nothing()
            .execute(&mut conn)
            .await?;

        notifications::process_like(&post, &actor, false, &data.db_pool).await?;

        Ok(())
    }
}
