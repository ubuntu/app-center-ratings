use std::{env, fmt::Display, str::FromStr};

use dotenv::dotenv;
use log::debug;
use thiserror::Error;

const ENV_APP_NAME: &str = "APP";
const ENV_NAME_ENV: &str = "ENV";
const ENV_NAME_LOG_LEVEL: &str = "RUST_LOG";
const ENV_NAME_PORT: &str = "PORT";

const DEFAULT_DEV_PORT: &str = "18080";

pub const ENV_NAME_DEV: &str = "dev";
pub const ENV_NAME_BETA: &str = "beta";
pub const ENV_NAME_STABLE: &str = "stable";

pub fn init() {
    dotenv().ok();
}

pub fn print_env_if_dev() {
    if get_env_name() == EnvName::Dev {
        debug!("Environment:");
        let env = env::vars();
        for (key, value) in env {
            debug!("{key}: {value}");
        }
    }
}

pub fn get_log_level() -> String {
    let value = env::var(ENV_NAME_LOG_LEVEL).unwrap();
    value
}

pub fn get_env_name() -> EnvName {
    let value = env::var(ENV_NAME_ENV).unwrap_or(ENV_NAME_STABLE.to_string());
    EnvName::from_str(&value).unwrap()
}

pub fn get_port() -> u16 {
    env::var(ENV_NAME_PORT)
        .unwrap_or(DEFAULT_DEV_PORT.to_string())
        .parse()
        .unwrap()
}

#[derive(PartialEq)]
pub enum EnvName {
    Dev,
    Beta,
    Stable,
}

impl Display for EnvName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EnvName::Dev => write!(f, "{ENV_NAME_DEV}"),
            EnvName::Beta => write!(f, "{ENV_NAME_BETA}"),
            EnvName::Stable => write!(f, "{ENV_NAME_STABLE}"),
        }
    }
}

#[derive(Error, Debug)]
pub enum EnvError {
    #[error("Unknown environment: {0}")]
    UnknownEnvNameError(String),
}

impl FromStr for EnvName {
    type Err = EnvError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let value = s.to_lowercase();
        let value = value.as_str();

        match value {
            ENV_NAME_DEV => Ok(EnvName::Dev),
            ENV_NAME_BETA => Ok(EnvName::Beta),
            ENV_NAME_STABLE => Ok(EnvName::Stable),
            unknown => Err(EnvError::UnknownEnvNameError(unknown.to_string())),
        }
    }
}
