use dotenv::dotenv;
use once_cell::sync::OnceCell;
use serde::Deserialize;
use tracing::info;

static CONFIG: OnceCell<Config> = OnceCell::new();

#[derive(Deserialize, Debug)]
pub struct Config {
    pub env: String,
    pub host: String,
    pub jwt_secret: String,
    pub log_level: String,
    pub name: String,
    pub port: u16,
    pub postgres_uri: String,
}

pub fn init() {
    dotenv().ok();
    match envy::prefixed("APP_").from_env::<Config>() {
        Ok(config) => {
            if config.env != "PRD" {
                info!("App config {:#?}", config);
            }
            CONFIG.set(config).expect("Failed to set CONFIG");
        }
        Err(error) => {
            panic!("Invalid app config: {:#?}", error)
        }
    }
}

pub fn get_config() -> &'static Config {
    CONFIG.get().expect("Config should be initialised")
}

pub fn get_socket() -> String {
    let Config { port, host, .. } = get_config();
    format!("{host}:{port}")
}
