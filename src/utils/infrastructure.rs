//! Utilities and structs for creating server infrastructure (database, etc).
use std::{
    error::Error,
    fmt::{Debug, Formatter},
    sync::Arc,
};

use snapd::SnapdClient;
use sqlx::{pool::PoolConnection, postgres::PgPoolOptions, PgPool, Postgres};

use crate::utils::{config::Config, jwt::Jwt};

/// Resources important to the server, but are not necessarily in-memory
#[derive(Clone)]
pub struct Infrastructure {
    /// The postgres DB
    pub postgres: Arc<PgPool>,
    /// The client for making snapd requests
    pub snapd_client: SnapdClient,
    /// The JWT instance
    pub jwt: Arc<Jwt>,
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

        Ok(Infrastructure {
            postgres,
            jwt,
            snapd_client: Default::default(),
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
