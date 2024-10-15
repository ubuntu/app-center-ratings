//! Utilities and structs for creating server infrastructure (database, etc).
use std::{
    error::Error,
    fmt::{Debug, Formatter},
    sync::Arc,
};

use sqlx::{pool::PoolConnection, postgres::PgPoolOptions, PgPool, Postgres};
use tokio::sync::OnceCell;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{reload::Handle, Registry};

use super::{jwt::JwtEncoder, log_util};
use crate::utils::config::Config;

/// The global reload handle, since [`tracing_subscriber`] is we have to be too because it panics
/// if you call init twice, which makes it so tests can't initialize [`Infrastructure`] more than once.
static RELOAD_HANDLE: tokio::sync::OnceCell<Handle<LevelFilter, Registry>> = OnceCell::const_new();

/// Resources important to the server, but are not necessarily in-memory
#[derive(Clone)]
pub struct Infrastructure {
    /// The postgres DB
    pub postgres: Arc<PgPool>,
    /// The reload handle for the logger
    pub log_reload_handle: &'static Handle<LevelFilter, Registry>,
    /// The utility which lets us encode user tokens with our JWT credentials
    pub jwt_encoder: Arc<JwtEncoder>,
}

impl Infrastructure {
    /// Tries to create a new [`Infrastructure`] from the given [`Config`]
    pub async fn new(config: &Config) -> Result<Infrastructure, Box<dyn Error>> {
        let uri = config.postgres_uri.clone();
        let uri = uri.as_str();

        let postgres = PgPoolOptions::new().max_connections(5).connect(uri).await?;
        let postgres = Arc::new(postgres);

        let jwt_encoder = JwtEncoder::from_config(config)?;
        let jwt_encoder = Arc::new(jwt_encoder);

        let reload_handle = RELOAD_HANDLE
            .get_or_try_init(|| async move { log_util::init_logging(&config.log_level) })
            .await?;

        Ok(Infrastructure {
            postgres,
            jwt_encoder,
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
