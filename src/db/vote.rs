use crate::db::{categories::Category, ClientHash, Error, Result};
use cached::proc_macro::cached;
use sqlx::{types::time::OffsetDateTime, FromRow, PgConnection, QueryBuilder};
use tracing::error;

/// A Vote, as submitted by a user
#[derive(Debug, Clone, FromRow, PartialEq, Eq)]
pub struct Vote {
    /// The hash of the user client
    pub client_hash: ClientHash,
    /// The ID of the snap being voted on
    pub snap_id: String,
    /// The revision of the snap being voted on
    #[sqlx(try_from = "i32")]
    pub snap_revision: u32,
    /// Whether this is a positive or negative vote
    pub vote_up: bool,
    /// The timestamp of the vote
    #[sqlx(rename = "created")]
    pub timestamp: OffsetDateTime,
}

impl Vote {
    /// Gets votes for a snap with the given ID from a given [`ClientHash`]
    ///
    /// [`ClientHash`]: crate::db::ClientHash
    pub async fn get_all_by_client_hash(
        client_hash: &str,
        snap_id_filter: Option<String>,
        conn: &mut PgConnection,
    ) -> Result<Vec<Vote>> {
        let votes = sqlx::query_as(
            r#"
                SELECT
                    votes.id,
                    votes.created,
                    votes.snap_id,
                    votes.snap_revision,
                    votes.vote_up,
                    users.client_hash
                FROM
                    users
                INNER JOIN
                    votes
                ON
                    users.id = votes.user_id_fk
                WHERE
                    users.client_hash = $1
                AND
                    ($2 IS NULL OR votes.snap_id = $2);
        "#,
        )
        .bind(client_hash)
        .bind(snap_id_filter)
        .fetch_all(conn)
        .await
        .map_err(|error| {
            error!("{error:?}");
            Error::FailedToGetUserVote
        })?;

        Ok(votes)
    }

    /// Saves a [`Vote`] to the database, if possible.
    pub async fn save_to_db(self, conn: &mut PgConnection) -> Result<u64> {
        let result = sqlx::query(
            r#"
        INSERT INTO votes (user_id_fk, snap_id, snap_revision, vote_up)
        VALUES ((SELECT id FROM users WHERE client_hash = $1), $2, $3, $4)
        ON CONFLICT (user_id_fk, snap_id, snap_revision)
        DO UPDATE SET vote_up = EXCLUDED.vote_up;
        "#,
        )
        .bind(self.client_hash)
        .bind(self.snap_id)
        .bind(self.snap_revision as i32)
        .bind(self.vote_up)
        .execute(conn)
        .await
        .map_err(|error| {
            error!("{error:?}");
            Error::FailedToCastVote
        })?;

        Ok(result.rows_affected())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::FromRepr)]
#[repr(i32)]
pub enum Timeframe {
    Unspecified,
    Week,
    Month,
}

/// A summary of votes for a given snap, this is then aggregated before transfer.
#[derive(Debug, Clone, FromRow)]
pub struct VoteSummary {
    /// The ID of the snap being checked.
    pub snap_id: String,
    /// The total votes this snap has received.
    pub total_votes: i64,
    /// The number of the votes which are positive.
    pub positive_votes: i64,
}

impl VoteSummary {
    pub async fn get_by_snap_id(snap_id: &str, conn: &mut PgConnection) -> Result<VoteSummary> {
        get_by_snap_id_cached(snap_id, conn).await
    }

    pub async fn get_by_snap_ids(
        snap_ids: &[String],
        timeframe: Timeframe,
        conn: &mut PgConnection,
    ) -> Result<Vec<VoteSummary>> {
        if snap_ids.is_empty() {
            return Ok(vec![]);
        }

        let mut builder = QueryBuilder::new(
            r#"
            SELECT
                votes.snap_id,
                COUNT(*) AS total_votes,
                COUNT(*) FILTER (WHERE votes.vote_up) AS positive_votes
            FROM
                votes
            WHERE
                votes.snap_id = ANY($1)
        "#,
        );

        builder.push(match timeframe {
            Timeframe::Week => " AND votes.created >= NOW() - INTERVAL '1 week'",
            Timeframe::Month => " AND votes.created >= NOW() - INTERVAL '1 month'",
            Timeframe::Unspecified => "",
        });

        builder.push(" GROUP BY votes.snap_id");

        let summaries = builder
            .build_query_as()
            .bind(snap_ids)
            .fetch_all(conn)
            .await?;

        Ok(summaries)
    }

    /// Retrieves the vote summary over a given [Timeframe], optionally for a specific [Category]
    pub async fn get_for_timeframe(
        timeframe: Timeframe,
        category: Option<Category>,
        conn: &mut PgConnection,
    ) -> Result<Vec<VoteSummary>> {
        let mut builder = QueryBuilder::new(
            r"
            SELECT
                votes.snap_id,
                COUNT(*) AS total_votes,
                COUNT(*) FILTER (WHERE votes.vote_up) AS positive_votes
            FROM
                votes",
        );

        builder.push(match timeframe {
            Timeframe::Week => " WHERE votes.created >= NOW() - INTERVAL '1 week'",
            Timeframe::Month => " WHERE votes.created >= NOW() - INTERVAL '1 month'",
            Timeframe::Unspecified => "",
        });

        if let Some(category) = category {
            builder
                .push(
                    r" 
                    WHERE votes.snap_id IN (
                    SELECT snap_categories.snap_id FROM snap_categories 
                    WHERE snap_categories.category = ",
                )
                .push_bind(category)
                .push(")");
        }

        builder.push(" GROUP BY votes.snap_id");
        let summaries = builder.build_query_as().fetch_all(conn).await?;

        Ok(summaries)
    }
}

#[cfg_attr(not(feature = "skip_cache"), cached(
    time = 86400, // 24 hours
    sync_writes = true,
    key = "String",
    convert = r##"{String::from(snap_id)}"##,
    result = true,
))]
async fn get_by_snap_id_cached(snap_id: &str, conn: &mut PgConnection) -> Result<VoteSummary> {
    let result: Option<VoteSummary> = sqlx::query_as(
        r#"
            SELECT
                votes.snap_id,
                COUNT(*) AS total_votes,
                COUNT(*) FILTER (WHERE votes.vote_up) AS positive_votes
            FROM
                votes
            WHERE
                votes.snap_id = $1
            GROUP BY votes.snap_id
        "#,
    )
    .bind(snap_id)
    .fetch_optional(conn)
    .await?;

    let summary = result.unwrap_or_else(|| VoteSummary {
        snap_id: snap_id.to_string(),
        total_votes: 0,
        positive_votes: 0,
    });

    Ok(summary)
}
