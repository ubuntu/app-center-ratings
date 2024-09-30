use sqlx::{postgres::PgPoolOptions, PgPool};
use thiserror::Error;
use tokio::sync::OnceCell;
use tracing::info;

use crate::utils::{Config, Migrator};

pub mod user;
pub mod vote;

pub type ClientHash = String;

/// Errors that can occur when a user votes.
#[derive(Error, Debug)]
pub enum UserError {
    /// A record could not be created for the user
    #[error("failed to create user record")]
    FailedToCreateUserRecord,
    /// We were unable to delete a user with the given instance ID
    #[error("failed to delete user by instance id")]
    FailedToDeleteUserRecord,
    /// We could not get a vote by a given user
    #[error("failed to get user vote")]
    FailedToGetUserVote,
    /// The user was unable to cast a vote
    #[error("failed to cast vote")]
    FailedToCastVote,
    /// An error that occurred in category updating
    #[error("an error occurred with the DB when getting categories: {0}")]
    CategoryDBError(#[from] sqlx::Error),
    /// Anything else that can go wrong
    #[error("unknown user error")]
    Unknown,
}

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
