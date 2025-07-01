use std::sync::Arc;

use activitypub_federation::{
    config::Data,
    fetch::object_id::ObjectId,
    protocol::{public_key::PublicKey, verification::verify_domains_match},
    traits::{Actor, Object},
};
use anyhow::anyhow;
use chrono::{DateTime, Utc};
use db::{
    models::{User, UserInsert},
    schema::users,
    types::DbId,
};
use diesel::{insert_into, query_dsl::QueryDsl, result::Error::NotFound, ExpressionMethods};
use diesel_async::RunQueryDsl;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, NoneAsEmptyString};
use svix_ksuid::KsuidLike;
use url::Url;
use web::AppState;

db_to_ap!(db::models::User, ApUser);

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum UserTypes {
    Person,
    Service,
    Application,
    Organization,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Endpoints {
    pub shared_inbox: Url,
}

#[serde_as]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Person {
    #[serde(rename = "type")]
    pub kind: UserTypes,
    pub id: ObjectId<ApUser>,
    pub preferred_username: String,
    pub inbox: Url,
    pub outbox: Url,
    pub followers: Url,
    pub following: Url,
    pub public_key: PublicKey,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoints: Option<Endpoints>,
    #[serde(default)]
    pub manually_approves_followers: bool,

    /// displayname
    pub name: Option<String>,
    #[serde_as(as = "NoneAsEmptyString")]
    pub summary: Option<String>,
    pub published: Option<DateTime<Utc>>,
    pub updated: Option<DateTime<Utc>>,

    #[serde(default)]
    pub is_cat: bool,
}

#[async_trait::async_trait]
impl Object for ApUser {
    type DataType = Arc<AppState>;
    type Kind = Person;
    type Error = anyhow::Error;

    async fn read_from_id(
        object_id: Url,
        data: &Data<Self::DataType>,
    ) -> Result<Option<Self>, Self::Error> {
        let user = users::table
            .filter(users::ap_id.eq(object_id.to_string()))
            .first::<db::models::User>(&mut data.db_pool.get().await?)
            .await;
        match user {
            Ok(user) => Ok(Some(ApUser(user))),
            Err(NotFound) => Ok(None),
            Err(err) => Err(err.into()),
        }
    }

    async fn into_json(self, _data: &Data<Self::DataType>) -> Result<Self::Kind, Self::Error> {
        let ap_id = self.ap_id.clone();
        let bio = self.bio.clone();
        let updated = self.updated;
        let published = self.published;
        let name = self.name.clone();
        Ok(Person {
            kind: if self.bot {
                UserTypes::Service
            } else {
                UserTypes::Person
            },
            id: ObjectId::from(Url::parse(&self.ap_id)?),
            name: self.display_name.clone().or(Some(name)),
            preferred_username: self.name.clone(),
            inbox: Url::parse(&self.inbox_uri)?,
            outbox: Url::parse(&self.outbox_uri)?,
            public_key: self.public_key(),
            summary: bio.or(Some("".to_string())),
            updated,
            published: Some(published),
            endpoints: if let Some(shared_inbox_uri) = &self.shared_inbox_uri {
                Some(Endpoints {
                    shared_inbox: Url::parse(shared_inbox_uri)?,
                })
            } else {
                None
            },
            manually_approves_followers: self.manually_approves_followers,
            is_cat: self.is_cat,
            followers: Url::parse(&(ap_id.clone() + "/ap/followers"))?, // TODO
            following: Url::parse(&(ap_id + "/ap/following"))?,         // TODO
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

        let user = UserInsert {
            id: DbId::from(svix_ksuid::Ksuid::new(
                json.published
                    .map(|f| time::OffsetDateTime::from_unix_timestamp(f.timestamp()).unwrap()),
                None,
            )),
            ap_id: json.id.to_string(),
            local: false,
            inbox_uri: json.inbox.to_string(),
            shared_inbox_uri: json.endpoints.map(|e| e.shared_inbox.to_string()),
            outbox_uri: json.outbox.to_string(),
            followers_uri: json.followers.to_string(),
            name: json.preferred_username,
            instance: match json.id.inner().host() {
                None => return Err(anyhow!("json id host is None")),
                Some(id) => match id {
                    url::Host::Domain(s) => s.to_string(),
                    _ => return Err(anyhow!("json id host cannot be an IP")),
                },
            },
            display_name: json.name,
            bio: json.summary,
            password_encrypted: None,
            admin: false,
            public_key: json.public_key.public_key_pem,
            private_key: None,
            published: json.updated.unwrap_or(Utc::now()),
            updated: Some(Utc::now()),
            manually_approves_followers: json.manually_approves_followers,
            is_cat: json.is_cat,
            bot: json.kind == UserTypes::Service || json.kind == UserTypes::Application,
        };

        Ok(ApUser(
            insert_into(users::table)
                .values(user.clone())
                .on_conflict(users::ap_id)
                .do_update()
                .set(user)
                .get_result::<User>(&mut conn)
                .await?,
        ))
    }
}

impl Actor for ApUser {
    fn id(&self) -> Url {
        Url::parse(&self.ap_id).unwrap() // should never panic in theory
    }

    fn public_key_pem(&self) -> &str {
        &self.public_key
    }

    fn private_key_pem(&self) -> Option<String> {
        self.private_key.clone()
    }

    fn inbox(&self) -> Url {
        Url::parse(&self.inbox_uri).unwrap() // should never panic in theory
    }

    fn shared_inbox(&self) -> Option<Url> {
        self.shared_inbox_uri
            .as_ref()
            .map(|shared_inbox_uri| Url::parse(shared_inbox_uri).unwrap()) // should never panic in theory
    }
}
