use db::models::Application as DbApplication;
use serde::Serialize;
use serde_with::skip_serializing_none;

// TODO: Fully implement https://docs.joinmastodon.org/entities/Application/
#[skip_serializing_none]
#[derive(Serialize, Debug)]
pub struct Application {
    pub id: String,
    pub name: String,
    #[serialize_always]
    pub website: Option<String>,
    pub redirect_uri: String,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
}

impl Application {
    pub fn new(application: DbApplication, show_secrets: bool) -> Self {
        if show_secrets {
            Self {
                id: application.id.to_string(),
                name: application.name,
                website: application.website,
                redirect_uri: application.redirect_url,
                client_id: Some(application.client_id),
                client_secret: Some(application.client_secret),
            }
        } else {
            Self {
                id: application.id.to_string(),
                name: application.name,
                website: application.website,
                redirect_uri: application.redirect_url,
                client_id: None,
                client_secret: None,
            }
        }
    }
}
