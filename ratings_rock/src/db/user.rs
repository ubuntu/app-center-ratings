use crate::db::{ClientHash, Error, Result};
use sqlx::{prelude::FromRow, types::time::OffsetDateTime, PgConnection};
use tracing::error;

/// Information about a user who may be rating snaps.
#[derive(Debug, FromRow)]
pub struct User {
    /// The user's ID
    pub id: i32,
    /// A hash of the user's client
    pub client_hash: ClientHash,
    /// The time the user was created
    pub created: OffsetDateTime,
    /// The time the user was last seen
    pub last_seen: OffsetDateTime,
}

impl User {
    /// Create a [`User`] entry, or note that the user has recently been seen
    pub async fn create_or_seen(client_hash: &str, conn: &mut PgConnection) -> Result<Self> {
        let user_with_id = sqlx::query_as(
            r#"
        INSERT INTO users (client_hash, created, last_seen)
        VALUES ($1, NOW(), NOW())
        ON CONFLICT (client_hash)
        DO UPDATE SET last_seen = NOW()
        RETURNING id, client_hash, created, last_seen;
        "#,
        )
        .bind(client_hash)
        .fetch_one(conn)
        .await
        .map_err(|error| {
            error!("{error:?}");
            Error::FailedToCreateUserRecord
        })?;

        Ok(user_with_id)
    }

    pub async fn delete_by_client_hash(client_hash: &str, conn: &mut PgConnection) -> Result<()> {
        sqlx::query(
            r#"
        DELETE FROM users
        WHERE client_hash = $1
        "#,
        )
        .bind(client_hash)
        .execute(conn)
        .await
        .map_err(|error| {
            error!("{error:?}");
            Error::FailedToDeleteUserRecord
        })?;

        Ok(())
    }
}
