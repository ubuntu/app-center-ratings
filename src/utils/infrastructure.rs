//! Utilities and structs for creating server infrastructure (database, etc).
use std::{
    error::Error,
    fmt::{Debug, Formatter},
    sync::Arc,
};

use snapd::SnapdClient;
use sqlx::{pool::PoolConnection, postgres::PgPoolOptions, PgPool, Postgres};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{reload::Handle, Registry};

use crate::utils::{config::Config, jwt::Jwt};

use super::log_util;

/// Resources important to the server, but are not necessarily in-memory
#[derive(Clone)]
pub struct Infrastructure {
    /// The postgres DB
    pub postgres: Arc<PgPool>,
    /// The client for making snapd requests
    pub snapd_client: SnapdClient,
    /// The JWT instance
    pub jwt: Arc<Jwt>,
    /// The reload handle for the logger
    pub log_reload_handle: Handle<LevelFilter, Registry>,
}

impl Infrastructure {
    /// Tries to create a new [`Infrastructure`] from the given [`Config`]
    pub async fn new(config: &Config) -> Result<Infrastructure, Box<dyn Error>> {
        let uri = config.postgres_uri.clone();
        let uri = uri.as_str();

        let postgres = PgPoolOptions::new().max_connections(5).connect(uri).await?;
        let postgres = Arc::new(postgres);

        let jwt = Jwt::new(&config.jwt_secret)?;
        let jwt = Arc::new(jwt);

        let reload_handle = log_util::init_logging(&config.log_level)?;

        Ok(Infrastructure {
            postgres,
            jwt,
            snapd_client: Default::default(),
            log_reload_handle: reload_handle,
        })
    }

    /// Attempt to get a pooled connection to the database
    pub async fn repository(&self) -> Result<PoolConnection<Postgres>, sqlx::Error> {
        self.postgres.acquire().await
    }
}

impl Debug for Infrastructure {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("Infrastructure { postgres, jwt }")
    }
}
