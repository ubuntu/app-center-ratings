//! Utility functions and definitions for configuring the service.
use dotenvy::dotenv;
use serde::Deserialize;

/// Configuration for the general app center ratings backend service.
#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    /// Environment variables to use
    pub env: String,
    /// The host configuration
    pub host: String,
    /// The JWT secret value
    pub jwt_secret: String,
    /// Log level to use
    pub log_level: String,
    /// The service name
    pub name: String,
    /// The port to run on
    pub port: u16,
    /// The URI of the postgres database
    pub postgres_uri: String,
    /// The URI of the migration resource for the DB
    pub migration_postgres_uri: String,
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
