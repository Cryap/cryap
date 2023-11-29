use std::sync::Arc;

use activitypub_federation::{
    config::Data,
    fetch::object_id::ObjectId,
    kinds::activity::AnnounceType,
    protocol::{helpers::deserialize_one_or_many, verification::verify_domains_match},
    traits::Object,
};
use chrono::{DateTime, Utc};
use db::{models::PostBoost, schema::post_boost, types::DbId};
use diesel::{insert_into, query_dsl::QueryDsl, result::Error::NotFound, ExpressionMethods};
use diesel_async::RunQueryDsl;
use serde::{Deserialize, Serialize};
use svix_ksuid::KsuidLike;
use url::Url;
use web::AppState;

use super::{note::ApNote, user::ApUser};
use crate::objects::note::{construct_to_cc, parse_to_cc};

db_to_ap!(db::models::PostBoost, ApAnnounce);

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Announce {
    #[serde(rename = "type")]
    pub kind: AnnounceType,
    pub id: ObjectId<ApAnnounce>,

    pub actor: ObjectId<ApUser>,
    pub object: ObjectId<ApNote>,
    pub published: Option<DateTime<Utc>>,

    #[serde(deserialize_with = "deserialize_one_or_many")]
    pub to: Vec<Url>,
    #[serde(deserialize_with = "deserialize_one_or_many", default)]
    pub cc: Vec<Url>,
}

#[async_trait::async_trait]
impl Object for ApAnnounce {
    type DataType = Arc<AppState>;
    type Kind = Announce;
    type Error = anyhow::Error;

    async fn read_from_id(
        object_id: Url,
        data: &Data<Self::DataType>,
    ) -> Result<Option<Self>, Self::Error> {
        let user = post_boost::table
            .filter(post_boost::ap_id.eq(object_id.to_string()))
            .first::<db::models::PostBoost>(&mut data.db_pool.get().await?)
            .await;
        match user {
            Ok(post) => Ok(Some(ApAnnounce(post))),
            Err(NotFound) => Ok(None),
            Err(err) => Err(err.into()),
        }
    }

    async fn into_json(self, data: &Data<Self::DataType>) -> Result<Self::Kind, Self::Error> {
        let actor = &self.author(&data.db_pool).await?;
        let (to, cc) = construct_to_cc(&self.visibility, Url::parse(&actor.followers_uri)?, vec![]);

        Ok(Announce {
            kind: Default::default(),
            id: ObjectId::from(Url::parse(&self.ap_id)?),
            actor: ObjectId::<ApUser>::from(Url::parse(&actor.ap_id)?),
            object: ObjectId::<ApNote>::from(Url::parse(&self.post(&data.db_pool).await?.ap_id)?),
            published: Some(self.published),
            to,
            cc,
        })
    }

    async fn verify(
        json: &Self::Kind,
        expected_domain: &Url,
        _data: &Data<Self::DataType>,
    ) -> Result<(), Self::Error> {
        verify_domains_match(json.id.inner(), expected_domain)?;
        Ok(())
    }

    async fn from_json(json: Self::Kind, data: &Data<Self::DataType>) -> Result<Self, Self::Error> {
        let mut conn = data.db_pool.get().await?;

        let actor = json.actor.dereference(data).await?;
        let post = json.object.dereference(data).await?;

        let boost = PostBoost {
            id: DbId::from(svix_ksuid::Ksuid::new(
                json.published
                    .map(|f| time::OffsetDateTime::from_unix_timestamp(f.timestamp()).unwrap()),
                None,
            )),
            actor_id: actor.id.clone(),
            post_id: post.id.clone(),
            ap_id: json.id.to_string(),
            visibility: parse_to_cc(&json.to, &json.cc, Url::parse(&actor.followers_uri)?),
            published: Utc::now(),
        };

        let boost_db = insert_into(post_boost::table)
            .values(boost.clone())
            .on_conflict(post_boost::ap_id)
            .do_update()
            .set(boost)
            .get_result::<PostBoost>(&mut conn)
            .await?;

        Ok(ApAnnounce(boost_db))
    }
}
