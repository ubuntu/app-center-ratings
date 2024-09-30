use sqlx::{postgres::PgPoolOptions, PgPool};
use tokio::sync::OnceCell;
use tracing::info;

use crate::utils::{Config, Migrator};

pub mod user;
pub mod vote;

const MAX_POOL_CONNECTIONS: u32 = 5;

static POOL: OnceCell<PgPool> = OnceCell::const_new();

pub async fn init_pool_from_uri(postgres_uri: &str) -> Result<PgPool, sqlx::Error> {
    info!("Initialising DB connection pool");
    let pool = PgPoolOptions::new()
        .max_connections(MAX_POOL_CONNECTIONS)
        .connect(postgres_uri)
        .await?;

    Ok(pool)
}

pub async fn init_pool_from_uri_and_migrate(
    postgres_uri: &str,
) -> Result<PgPool, Box<dyn std::error::Error>> {
    let pool = init_pool_from_uri(postgres_uri).await?;
    info!("Running DB migrations");
    let migrator = Migrator::new(postgres_uri).await?;
    migrator.run().await?;

    Ok(pool)
}

pub async fn get_pool() -> Result<&'static PgPool, Box<dyn std::error::Error>> {
    let config = Config::load()?;
    let pool = POOL
        .get_or_try_init(|| init_pool_from_uri_and_migrate(&config.postgres_uri))
        .await?;
    Ok(pool)
}

#[macro_export]
macro_rules! conn {
    { } => {
        &mut *($crate::db::get_pool().await?.acquire().await?)
    }
}
