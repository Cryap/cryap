use web::config::Config;

pub fn process_config() -> anyhow::Result<Config> {
    let config = std::fs::read_to_string("config.toml")?;
    Ok(toml::from_str(&config)?)
}
