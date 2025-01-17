//! Utility functions and definitions for configuring the service.
use dotenvy::dotenv;
use secrecy::SecretString;
use serde::Deserialize;

/// Configuration for the general app center ratings backend service.
#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    /// The host configuration
    pub host: String,
    /// The port to run on
    pub port: u16,
    /// The URI of the postgres database
    pub postgres_uri: String,
    /// The JWT secret value
    pub jwt_secret: SecretString,
    /// The base URI for snapcraft.io
    pub snapcraft_io_uri: String,
    /// The path to the tls keychain
    pub tls_keychain_path: Option<String>,
    /// The path to the tls private key
    pub tls_key_path: Option<String>,
}

impl Config {
    /// Loads the configuration from environment variables
    pub fn load() -> envy::Result<Config> {
        dotenv().ok();

        envy::prefixed("APP_").from_env::<Config>()
    }

    /// Return a [`String`] representing the socket to run the service on
    pub fn socket(&self) -> String {
        let Config { port, host, .. } = self;

        format!("{host}:{port}")
    }
}
