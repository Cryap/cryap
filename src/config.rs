use web::config::Config;

pub fn process_config() -> Result<Config, anyhow::Error> {
    let config = std::fs::read_to_string("config.toml")?;
    Ok(toml::from_str(&config)?)
}
