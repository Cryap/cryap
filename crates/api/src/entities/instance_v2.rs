use serde::Serialize;
use web::config::Config;

use crate::entities::Rule;

// TODO: Fully implement https://docs.joinmastodon.org/entities/Instance/

#[derive(Serialize, Debug)]
pub struct ConfigurationUrls {
    pub streaming: String,
}

#[derive(Serialize, Debug)]
pub struct ConfigurationStatuses {
    pub max_characters: i32,
}

#[derive(Serialize, Debug)]
pub struct Configuration {
    pub urls: ConfigurationUrls,
    pub statuses: ConfigurationStatuses,
}

#[derive(Serialize, Debug)]
pub struct Registrations {
    pub enabled: bool,
    pub approval_required: bool,
    pub message: Option<String>,
}

#[derive(Serialize, Debug)]
pub struct Instance {
    pub domain: String,
    pub title: String,
    pub version: String,
    pub cryap_version: String,
    pub source_url: String,
    pub description: String,
    pub thumbnail: Option<String>,
    pub languages: Vec<String>,
    pub configuration: Configuration,
    pub registrations: Registrations,
    pub rules: Vec<Rule>,
}

impl Instance {
    pub fn new(config: &Config) -> Self {
        Self {
            domain: config.web.domain.clone(),
            title: config.instance.title.clone(),
            version: String::from("4.1.5"), // Latest version of Mastodon. Specified for compatibility. TODO: Move it somewhere for convenient change
            cryap_version: String::from(env!("CARGO_PKG_VERSION")),
            source_url: String::from(env!("CARGO_PKG_REPOSITORY")),
            description: config.instance.description.clone(),
            thumbnail: None,
            languages: config.instance.languages.clone(),
            configuration: Configuration {
                urls: ConfigurationUrls {
                    streaming: format!("wss://{}", &config.web.domain),
                },
                statuses: ConfigurationStatuses {
                    max_characters: config.instance.max_characters,
                },
            },
            registrations: Registrations {
                enabled: false,
                approval_required: true,
                message: None,
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
