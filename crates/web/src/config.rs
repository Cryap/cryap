use serde::Deserialize;

#[derive(Clone, Deserialize)]
pub struct Config {
    pub web: Web,
    pub database: Database,
    pub redis: Redis,
}

#[derive(Clone, Deserialize)]
pub struct Web {
    pub domain: String,
    pub port: u16,
    #[serde(default = "host_default")]
    pub host: String,
}

#[derive(Clone, Deserialize)]
pub struct Database {
    pub uri: String,
}

#[derive(Clone, Deserialize)]
pub struct Redis {
    pub uri: String,
}

fn host_default() -> String {
    String::from("0.0.0.0")
}
