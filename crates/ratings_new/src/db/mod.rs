use crate::Config;
use sqlx::{postgres::PgPoolOptions, PgPool};
use thiserror::Error;
use tokio::sync::OnceCell;
use tracing::info;

mod categories;
mod user;
mod vote;

pub use categories::{set_categories_for_snap, snap_has_categories, Category};
pub use user::User;
pub use vote::{Timeframe, Vote, VoteSummary};

#[macro_export]
macro_rules! conn {
    { } => {
        &mut *($crate::db::get_pool().await?.acquire().await.map_err($crate::db::Error::from)?)
    }
}

pub type ClientHash = String;
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("failed to create user record")]
    FailedToCreateUserRecord,

    #[error("failed to delete user by instance id")]
    FailedToDeleteUserRecord,

    #[error("failed to get user vote")]
    FailedToGetUserVote,

    #[error("failed to cast vote")]
    FailedToCastVote,

    #[error(transparent)]
    Migration(#[from] sqlx::migrate::MigrateError),

    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),

    #[error(transparent)]
    Envy(#[from] envy::Error),
}

const MAX_POOL_CONNECTIONS: u32 = 5;

static POOL: OnceCell<PgPool> = OnceCell::const_new();

pub async fn init_pool_from_uri(postgres_uri: &str) -> Result<PgPool> {
    info!("Initialising DB connection pool");
    let pool = PgPoolOptions::new()
        .max_connections(MAX_POOL_CONNECTIONS)
        .connect(postgres_uri)
        .await?;

    Ok(pool)
}

pub async fn init_pool_from_uri_and_migrate() -> Result<PgPool> {
    let config = Config::load()?;
    let pool = init_pool_from_uri(&config.postgres_uri).await?;
    info!("Running DB migrations");
    sqlx::migrate!("sql/migrations").run(&pool).await?;

    Ok(pool)
}

pub async fn get_pool() -> Result<&'static PgPool> {
    let pool = POOL.get_or_try_init(init_pool_from_uri_and_migrate).await?;

    Ok(pool)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conn;
    use sqlx::types::time::OffsetDateTime;
    use tracing_subscriber::EnvFilter;

    #[cfg_attr(not(feature = "db_tests"), ignore)]
    #[tokio::test]
    async fn save_and_read_votes() -> Result<()> {
        let client_hash_1 = "0000000000000000000000000000000000000000000000000000000000000001";
        let client_hash_2 = "0000000000000000000000000000000000000000000000000000000000000002";
        let snap_id_1 = "00000000000000000000000000000001";
        let snap_id_2 = "00000000000000000000000000000002";

        tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::from_default_env())
            .init();

        let test_users = [client_hash_1, client_hash_2];

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

        let conn = conn!();

        for client_hash in test_users.into_iter() {
            User::create_or_seen(client_hash, conn).await?;
        }

        for vote in test_votes.into_iter() {
            vote.save_to_db(conn).await?;
        }

        let votes_client_1 =
            vote::Vote::get_all_by_client_hash(client_hash_1, Some(String::from(snap_id_1)), conn)
                .await
                .unwrap();

        let votes_client_2 =
            vote::Vote::get_all_by_client_hash(client_hash_2, Some(String::from(snap_id_2)), conn)
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

    #[cfg_attr(not(feature = "db_tests"), ignore)]
    #[tokio::test]
    async fn update_categories() -> Result<()> {
        let conn = conn!();
        let snap_id = "00000000000000000000000000000001";
        let cats = vec![categories::Category::ArtAndDesign];

        assert!(!categories::snap_has_categories(snap_id, conn)
            .await
            .unwrap());
        categories::set_categories_for_snap(snap_id, cats, conn)
            .await
            .unwrap();
        assert!(categories::snap_has_categories(snap_id, conn)
            .await
            .unwrap());

        Ok(())
    }
}
