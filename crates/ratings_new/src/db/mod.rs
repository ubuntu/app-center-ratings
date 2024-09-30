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

#[cfg(test)]
mod test {
    use sqlx::types::time::OffsetDateTime;
    use tracing_subscriber::EnvFilter;

    use crate::conn;

    use super::*;

    #[tokio::test]
    async fn save_and_read_votes() -> Result<(), Box<dyn std::error::Error>> {
        let client_hash_1 = "0000000000000000000000000000000000000000000000000000000000000001";
        let client_hash_2 = "0000000000000000000000000000000000000000000000000000000000000002";
        let snap_id_1 = "00000000000000000000000000000001";
        let snap_id_2 = "00000000000000000000000000000002";

        tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::from_default_env())
            .init();

        let test_users = [
            user::User::new(client_hash_1),
            user::User::new(client_hash_2),
        ];

        let test_votes = [
            vote::Vote {
                client_hash: String::from(client_hash_1),
                snap_id: String::from(snap_id_1),
                vote_up: true,
                timestamp: OffsetDateTime::from_unix_timestamp(123).unwrap(),
                snap_revision: 1,
            },
            vote::Vote {
                client_hash: String::from(client_hash_2),
                snap_id: String::from(snap_id_2),
                vote_up: false,
                timestamp: OffsetDateTime::from_unix_timestamp(456).unwrap(),
                snap_revision: 2,
            },
        ];

        let connection = conn!();

        for user in test_users.into_iter() {
            user.create_or_seen(connection).await?;
        }

        for vote in test_votes.into_iter() {
            vote.save_to_db(connection).await?;
        }

        let votes_client_1 = vote::Vote::get_all_by_client_hash_and_snap_id(
            connection,
            String::from(snap_id_1),
            String::from(client_hash_1),
        )
        .await
        .unwrap();

        let votes_client_2 = vote::Vote::get_all_by_client_hash_and_snap_id(
            connection,
            String::from(snap_id_2),
            String::from(client_hash_2),
        )
        .await
        .unwrap();

        assert_eq!(votes_client_1.len(), 1);
        let first_vote = votes_client_1.first().unwrap();
        assert_eq!(first_vote.snap_id, snap_id_1);
        assert_eq!(first_vote.client_hash, client_hash_1);
        assert_eq!(first_vote.snap_revision, 1);
        assert!(first_vote.vote_up);

        let second_vote = votes_client_2.first().unwrap();
        assert_eq!(votes_client_2.len(), 1);
        assert_eq!(second_vote.snap_id, snap_id_2);
        assert_eq!(second_vote.client_hash, client_hash_2);
        assert_eq!(second_vote.snap_revision, 2);
        assert!(!second_vote.vote_up);

        Ok(())
    }
}
