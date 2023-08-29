use dotenv::dotenv;
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub env: String,
    pub host: String,
    pub jwt_secret: String,
    pub log_level: String,
    pub name: String,
    pub port: u16,
    pub postgres_uri: String,
    pub migration_postgres_uri: String,
}

impl Config {
    pub fn load() -> envy::Result<Config> {
        dotenv().ok();
        envy::prefixed("APP_").from_env::<Config>()
    }

    pub fn socket(&self) -> String {
        let Config { port, host, .. } = self;
        format!("{host}:{port}")
    }
}
