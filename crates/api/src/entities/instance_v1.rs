use serde::Serialize;
use web::config::Config;

use crate::entities::Rule;

// TODO: Fully implement https://docs.joinmastodon.org/entities/V1_Instance/

#[derive(Serialize, Debug)]
pub struct Urls {
    pub streaming_api: String,
}

#[derive(Serialize, Debug)]
pub struct ConfigurationStatuses {
    pub max_characters: i32,
}

#[derive(Serialize, Debug)]
pub struct Configuration {
    pub statuses: ConfigurationStatuses,
}

#[derive(Serialize, Debug)]
pub struct Instance {
    pub uri: String,
    pub title: String,
    pub short_description: String,
    pub description: String,
    pub email: String,
    pub version: String,
    pub cryap_version: String,
    pub thumbnail: String,
    pub languages: Vec<String>,
    pub registrations: bool,
    pub approval_required: bool,
    pub invites_enabled: bool,
    pub urls: Urls,
    pub configuration: Configuration,
    pub rules: Vec<Rule>,
}

impl Instance {
    pub fn new(config: &Config) -> Self {
        Self {
            uri: config.web.domain.clone(),
            title: config.instance.title.clone(),
            short_description: String::new(),
            description: config.instance.description.clone(),
            email: String::new(),
            version: String::from("4.1.5"), // Latest version of Mastodon. Specified for compatibility. TODO: Move it somewhere for convenient change
            cryap_version: String::from(env!("CARGO_PKG_VERSION")),
            thumbnail: String::new(),
            languages: config.instance.languages.clone(),
            registrations: false,
            approval_required: true,
            invites_enabled: false,
            urls: Urls {
                streaming_api: format!("wss://{}", &config.web.domain),
            },
            configuration: Configuration {
                statuses: ConfigurationStatuses {
                    max_characters: config.instance.max_characters,
                },
            },
            rules: config
                .instance
                .rules
                .clone()
                .into_iter()
                .enumerate()
                .map(|(index, rule)| Rule {
                    id: (index + 1).to_string(),
                    text: rule,
                })
                .collect(),
        }
    }
}
