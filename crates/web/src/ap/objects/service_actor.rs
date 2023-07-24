use std::sync::Arc;

use activitypub_federation::{
    config::Data,
    fetch::object_id::ObjectId,
    http_signatures::Keypair,
    kinds::actor::ApplicationType,
    protocol::{public_key::PublicKey, verification::verify_domains_match},
    traits::{Actor, Object},
};
use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, NoneAsEmptyString};
use url::Url;

use super::user::Endpoints;
use crate::AppState;

#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceActor {
    #[serde(rename = "type")]
    pub(crate) kind: ApplicationType,
    pub(crate) id: ObjectId<ServiceActor>,
    pub(crate) preferred_username: String,
    pub(crate) inbox: Url,
    pub(crate) followers: Url,
    pub(crate) following: Url,
    pub(crate) public_key: PublicKey,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) endpoints: Option<Endpoints>,
    #[serde(skip)]
    pub(crate) private_key: String,

    /// displayname
    pub(crate) name: Option<String>,
    #[serde_as(as = "NoneAsEmptyString")]
    pub(crate) summary: Option<String>,
}

impl ServiceActor {
    pub fn new(url: Url, keypair: Keypair) -> ServiceActor {
        ServiceActor {
            kind: Default::default(),
            id: url.clone().into(),
            preferred_username: "service.actor".to_string(),
            inbox: Url::parse(&(url.to_string() + "/inbox")).unwrap(),
            followers: Url::parse(&(url.to_string() + "/followers")).unwrap(),
            following: Url::parse(&(url.to_string() + "/following")).unwrap(),
            public_key: PublicKey {
                id: url.to_string() + "#main-key",
                owner: url.clone(),
                public_key_pem: keypair.public_key,
            },
            endpoints: None,
            private_key: keypair.private_key.clone(),
            name: Some("Cryap".to_string()),
            summary: Some("An internal service actor".to_string()),
        }
    }
}

#[async_trait::async_trait]
impl Object for ServiceActor {
    type DataType = Arc<AppState>;
    type Kind = ServiceActor;
    type Error = anyhow::Error;

    async fn read_from_id(
        _object_id: Url,
        _data: &Data<Self::DataType>,
    ) -> Result<Option<Self>, Self::Error> {
        Err(anyhow!("ServiceActor - singleton"))
    }

    async fn into_json(self, _data: &Data<Self::DataType>) -> Result<Self::Kind, Self::Error> {
        Ok(self)
    }

    async fn verify(
        json: &Self::Kind,
        expected_domain: &Url,
        _data: &Data<Self::DataType>,
    ) -> Result<(), Self::Error> {
        verify_domains_match(json.id.inner(), expected_domain)?;
        Ok(())
    }

    async fn from_json(
        json: Self::Kind,
        _data: &Data<Self::DataType>,
    ) -> Result<Self, Self::Error> {
        Ok(json)
    }
}

impl Actor for ServiceActor {
    fn id(&self) -> Url {
        self.id.clone().into() // should never panic in theory
    }

    fn public_key_pem(&self) -> &str {
        &self.public_key.public_key_pem
    }

    fn private_key_pem(&self) -> Option<String> {
        Some(self.private_key.clone())
    }

    fn inbox(&self) -> Url {
        self.inbox.clone()
    }

    fn shared_inbox(&self) -> Option<Url> {
        self.endpoints.clone().map(|f| f.shared_inbox)
    }
}
