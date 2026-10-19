use crate::Config;
use sqlx::{postgres::PgPoolOptions, Connection, PgPool};
use thiserror::Error;
use tokio::sync::OnceCell;
use tracing::info;

mod categories;
#[cfg(test)]
mod migration_tests;
mod user;
mod vote;

pub use categories::{set_categories_for_snap, snap_has_categories, Category};
pub use user::User;
pub use vote::{PackageType, Timeframe, Vote, VoteSummary};

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

pub async fn check_db_conn() -> Result<()> {
    conn!().ping().await.map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::types::time::OffsetDateTime;
    use tracing_subscriber::EnvFilter;

    // Each #[tokio::test] runs on its own runtime, so tests must not share the
    // global POOL: its connections die with the runtime that opened them.
    async fn test_pool() -> Result<PgPool> {
        init_pool_from_uri_and_migrate().await
    }

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
                app_id: String::from(snap_id_1),
                package_type: PackageType::Snap,
                vote_up: true,
                timestamp: OffsetDateTime::from_unix_timestamp(123).unwrap(),
                snap_revision: 1,
            },
            vote::Vote {
                client_hash: String::from(client_hash_2),
                app_id: String::from(snap_id_2),
                package_type: PackageType::Snap,
                vote_up: false,
                timestamp: OffsetDateTime::from_unix_timestamp(456).unwrap(),
                snap_revision: 2,
            },
        ];

        let pool = test_pool().await?;
        let conn = &mut *pool.acquire().await?;

        for client_hash in test_users.into_iter() {
            User::create_or_seen(client_hash, conn).await?;
        }

        for vote in test_votes.into_iter() {
            vote.save_to_db(conn).await?;
        }

        let votes_client_1 = vote::Vote::get_all_by_client_hash(
            client_hash_1,
            PackageType::Snap,
            Some(String::from(snap_id_1)),
            conn,
        )
        .await
        .unwrap();

        let votes_client_2 = vote::Vote::get_all_by_client_hash(
            client_hash_2,
            PackageType::Snap,
            Some(String::from(snap_id_2)),
            conn,
        )
        .await
        .unwrap();

        assert_eq!(votes_client_1.len(), 1);
        let first_vote = votes_client_1.first().unwrap();
        assert_eq!(first_vote.app_id, snap_id_1);
        assert_eq!(first_vote.client_hash, client_hash_1);
        assert_eq!(first_vote.snap_revision, 1);
        assert!(first_vote.vote_up);

        let second_vote = votes_client_2.first().unwrap();
        assert_eq!(votes_client_2.len(), 1);
        assert_eq!(second_vote.app_id, snap_id_2);
        assert_eq!(second_vote.client_hash, client_hash_2);
        assert_eq!(second_vote.snap_revision, 2);
        assert!(!second_vote.vote_up);

        Ok(())
    }

    #[cfg_attr(not(feature = "db_tests"), ignore)]
    #[tokio::test]
    async fn snap_queries_ignore_deb_votes_for_same_app_id() -> Result<()> {
        let client_hash = "0000000000000000000000000000000000000000000000000000000000000003";
        let app_id = "00000000000000000000000000000003";

        let pool = test_pool().await?;
        let conn = &mut *pool.acquire().await?;
        User::create_or_seen(client_hash, conn).await?;

        vote::Vote {
            client_hash: String::from(client_hash),
            app_id: String::from(app_id),
            package_type: PackageType::Snap,
            vote_up: true,
            timestamp: OffsetDateTime::from_unix_timestamp(789).unwrap(),
            snap_revision: 3,
        }
        .save_to_db(conn)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO votes (user_id_fk, app_id, package_type, package_version, vote_up)
            VALUES ((SELECT id FROM users WHERE client_hash = $1), $2, $3, '1.0', false)
            ON CONFLICT DO NOTHING;
            "#,
        )
        .bind(client_hash)
        .bind(app_id)
        .bind(PackageType::Deb)
        .execute(&mut *conn)
        .await?;

        let votes = vote::Vote::get_all_by_client_hash(
            client_hash,
            PackageType::Snap,
            Some(String::from(app_id)),
            conn,
        )
        .await?;

        assert_eq!(votes.len(), 1);
        assert_eq!(votes[0].package_type, PackageType::Snap);
        assert_eq!(votes[0].snap_revision, 3);

        let summaries =
            VoteSummary::get_by_snap_ids(&[String::from(app_id)], Timeframe::Unspecified, conn)
                .await?;

        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].package_type, PackageType::Snap);
        assert_eq!(summaries[0].total_votes, 1);
        assert_eq!(summaries[0].positive_votes, 1);

        Ok(())
    }

    #[cfg_attr(not(feature = "db_tests"), ignore)]
    #[tokio::test]
    async fn update_categories() -> Result<()> {
        let pool = test_pool().await?;
        let conn = &mut *pool.acquire().await?;
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
