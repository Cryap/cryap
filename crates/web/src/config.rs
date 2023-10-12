use serde::Deserialize;

#[derive(Clone, Deserialize)]
pub struct Config {
    pub web: Web,
    pub database: Database,
    pub redis: Redis,
    pub instance: Instance,
}

#[derive(Clone, Deserialize)]
pub struct Web {
    pub domain: String,
    pub port: u16,
    #[serde(default = "host_default")]
    pub host: String,
}

fn host_default() -> String {
    String::from("0.0.0.0")
}

#[derive(Clone, Deserialize)]
pub struct Database {
    pub uri: String,
}

#[derive(Clone, Deserialize)]
pub struct Redis {
    pub uri: String,
}

#[derive(Clone, Deserialize)]
pub struct Instance {
    pub title: String,
    #[serde(default = "description_default")]
    pub description: String,
    #[serde(default = "languages_default")]
    pub languages: Vec<String>,
    pub rules: Vec<String>,
    #[serde(default = "max_characters_default")]
    pub max_characters: i32,
    #[serde(default = "display_name_max_characters_default")]
    pub display_name_max_characters: i32,
    #[serde(default = "bio_max_characters_default")]
    pub bio_max_characters: i32,
}

fn description_default() -> String {
    String::new()
}

fn languages_default() -> Vec<String> {
    vec![String::from("en")]
}

fn max_characters_default() -> i32 {
    200
}

fn display_name_max_characters_default() -> i32 {
    30
}

fn bio_max_characters_default() -> i32 {
    500
}
