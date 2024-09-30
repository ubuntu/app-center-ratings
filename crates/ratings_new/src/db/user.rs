use super::{ClientHash, DbError, Result};
use sqlx::{prelude::FromRow, types::time::OffsetDateTime, PgConnection};
use tracing::error;

/// Information about a user who may be rating snaps.
#[derive(Debug, Clone, FromRow)]
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
    /// Creates a new user from the given [`ClientHash`]
    pub fn new(client_hash: &str) -> Self {
        let now = OffsetDateTime::now_utc();
        Self {
            id: -1,
            client_hash: client_hash.to_string(),
            last_seen: now,
            created: now,
        }
    }

    /// Create a [`User`] entry, or note that the user has recently been seen
    pub async fn create_or_seen(self, connection: &mut PgConnection) -> Result<Self, DbError> {
        let user_with_id = sqlx::query_as(
            r#"
        INSERT INTO users (client_hash, created, last_seen)
        VALUES ($1, NOW(), NOW())
        ON CONFLICT (client_hash)
        DO UPDATE SET last_seen = NOW()
        RETURNING id, client_hash, created, last_seen;
        "#,
        )
        .bind(self.client_hash)
        .fetch_one(connection)
        .await
        .map_err(|error| {
            error!("{error:?}");
            DbError::FailedToCreateUserRecord
        })?;

        Ok(user_with_id)
    }

    pub async fn delete(self, connection: &mut PgConnection) -> Result<(), DbError> {
        sqlx::query(
            r#"
        DELETE FROM users
        WHERE client_hash = $1
        "#,
        )
        .bind(self.client_hash)
        .execute(connection)
        .await
        .map_err(|error| {
            error!("{error:?}");
            DbError::FailedToDeleteUserRecord
        })?;
        Ok(())
    }
}
