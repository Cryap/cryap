use std::sync::Arc;

use activitypub_federation::{
    config::Data,
    traits::{ActivityHandler, Object},
};
use async_trait::async_trait;
use url::Url;
use web::AppState;

pub use crate::objects::announce::Announce;
use crate::{activities::is_duplicate, common::notifications, objects::announce::ApAnnounce};

#[async_trait]
impl ActivityHandler for Announce {
    type DataType = Arc<AppState>;
    type Error = anyhow::Error;

    fn id(&self) -> &Url {
        self.id.inner()
    }

    fn actor(&self) -> &Url {
        self.actor.inner()
    }

    async fn verify(&self, data: &Data<Self::DataType>) -> Result<(), Self::Error> {
        ApAnnounce::verify(&self, &self.actor.inner(), data).await?;
        Ok(())
    }

    async fn receive(self, data: &Data<Self::DataType>) -> Result<(), Self::Error> {
        if is_duplicate(&self.id(), data).await? {
            return Ok(());
        }

        let actor = self.actor.dereference(data).await?;
        let post = self.object.dereference(data).await?;
        ApAnnounce::from_json(self, data).await?;
        notifications::process_boost(&post, &actor, false, &data.db_pool).await?;

        Ok(())
    }
}
