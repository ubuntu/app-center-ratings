use crate::db::{categories::Category, ClientHash, Error, Result};
use cached::proc_macro::cached;
use sqlx::{types::time::OffsetDateTime, FromRow, PgConnection, QueryBuilder};
use std::collections::HashMap;
use tracing::error;

/// The packaging format an App is distributed in.
///
/// Mirrors the `package_types` lookup table. Snap is the zero value on purpose,
/// see `docs/adr/0001-package-type-defaults-to-snap.md`.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, sqlx::Type)]
#[repr(i16)]
pub enum PackageType {
    #[default]
    Snap = 0,
    Deb = 1,
}

/// A Vote, as submitted by a user
#[derive(Debug, Clone, FromRow, PartialEq, Eq)]
pub struct Vote {
    /// The hash of the user client
    pub client_hash: ClientHash,
    /// The ID of the App being voted on
    pub app_id: String,
    /// The Package Type of the App being voted on
    pub package_type: PackageType,
    /// The revision of the snap being voted on, 0 for debs
    #[sqlx(try_from = "i32")]
    pub snap_revision: u32,
    /// The Package Version of the deb being voted on, `None` for snaps
    pub package_version: Option<String>,
    /// Whether this is a positive or negative vote
    pub vote_up: bool,
    /// The timestamp of the vote
    #[sqlx(rename = "created")]
    pub timestamp: OffsetDateTime,
}

impl Vote {
    /// Gets votes of the given [`PackageType`] from a given [`ClientHash`], optionally
    /// restricted to the App with the given ID
    ///
    /// [`ClientHash`]: crate::db::ClientHash
    pub async fn get_all_by_client_hash(
        client_hash: &str,
        package_type: PackageType,
        app_id_filter: Option<String>,
        conn: &mut PgConnection,
    ) -> Result<Vec<Vote>> {
        let votes = sqlx::query_as(
            r#"
                SELECT
                    votes.id,
                    votes.created,
                    votes.app_id,
                    votes.package_type,
                    COALESCE(votes.snap_revision, 0) AS snap_revision,
                    votes.package_version,
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
                    votes.package_type = $2
                AND
                    ($3 IS NULL OR votes.app_id = $3);
        "#,
        )
        .bind(client_hash)
        .bind(package_type)
        .bind(app_id_filter)
        .fetch_all(conn)
        .await
        .map_err(|error| {
            error!("{error:?}");
            Error::FailedToGetUserVote
        })?;

        Ok(votes)
    }

    /// Saves a [`Vote`] to the database, replacing the user's earlier vote on the same
    /// Snap Revision or Package Version.
    pub async fn save_to_db(self, conn: &mut PgConnection) -> Result<u64> {
        // The conflict target must match the partial unique index for the Package Type
        let (snap_revision, package_version, conflict_target) = match self.package_type {
            PackageType::Snap => (
                Some(self.snap_revision as i32),
                None,
                "(user_id_fk, app_id, snap_revision) WHERE package_type = 0",
            ),
            PackageType::Deb => (
                None,
                self.package_version,
                "(user_id_fk, app_id, package_version) WHERE package_type = 1",
            ),
        };

        let result = sqlx::query(&format!(
            r#"
        INSERT INTO votes (user_id_fk, app_id, package_type, snap_revision, package_version, vote_up)
        VALUES ((SELECT id FROM users WHERE client_hash = $1), $2, $3, $4, $5, $6)
        ON CONFLICT {conflict_target}
        DO UPDATE SET vote_up = EXCLUDED.vote_up;
        "#
        ))
        .bind(self.client_hash)
        .bind(self.app_id)
        .bind(self.package_type)
        .bind(snap_revision)
        .bind(package_version)
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

/// A summary of votes for a given App, this is then aggregated before transfer.
#[derive(Debug, Clone, FromRow)]
pub struct VoteSummary {
    /// The ID of the App being checked.
    pub app_id: String,
    /// The Package Type of the App being checked.
    pub package_type: PackageType,
    /// The total votes this App has received.
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
                votes.app_id,
                votes.package_type,
                COUNT(*) AS total_votes,
                COUNT(*) FILTER (WHERE votes.vote_up) AS positive_votes
            FROM
                votes
            WHERE
                votes.package_type = $1
            AND
                votes.app_id = ANY($2)
        "#,
        );

        builder.push(match timeframe {
            Timeframe::Week => " AND votes.created >= NOW() - INTERVAL '1 week'",
            Timeframe::Month => " AND votes.created >= NOW() - INTERVAL '1 month'",
            Timeframe::Unspecified => "",
        });

        builder.push(" GROUP BY votes.package_type, votes.app_id");

        let summaries = builder
            .build_query_as()
            .bind(PackageType::Snap)
            .bind(snap_ids)
            .fetch_all(conn)
            .await?;

        Ok(summaries)
    }

    /// Retrieves the all-time vote summaries of the given Apps in a single query, one per
    /// App in input order. Apps without votes get an empty summary.
    pub async fn get_by_apps(
        apps: &[(PackageType, String)],
        conn: &mut PgConnection,
    ) -> Result<Vec<VoteSummary>> {
        let (package_types, app_ids): (Vec<i16>, Vec<&str>) = apps
            .iter()
            .map(|(package_type, app_id)| (*package_type as i16, app_id.as_str()))
            .unzip();

        let found: Vec<VoteSummary> = sqlx::query_as(
            r#"
            SELECT
                votes.app_id,
                votes.package_type,
                COUNT(*) AS total_votes,
                COUNT(*) FILTER (WHERE votes.vote_up) AS positive_votes
            FROM
                votes
            WHERE
                (votes.package_type, votes.app_id) IN (
                    SELECT * FROM UNNEST($1::SMALLINT[], $2::TEXT[])
                )
            GROUP BY votes.package_type, votes.app_id
        "#,
        )
        .bind(package_types)
        .bind(app_ids)
        .fetch_all(conn)
        .await?;

        let found: HashMap<(PackageType, String), VoteSummary> = found
            .into_iter()
            .map(|summary| ((summary.package_type, summary.app_id.clone()), summary))
            .collect();

        Ok(apps
            .iter()
            .map(|(package_type, app_id)| {
                found
                    .get(&(*package_type, app_id.clone()))
                    .cloned()
                    .unwrap_or_else(|| VoteSummary {
                        app_id: app_id.clone(),
                        package_type: *package_type,
                        total_votes: 0,
                        positive_votes: 0,
                    })
            })
            .collect())
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
                votes.app_id,
                votes.package_type,
                COUNT(*) AS total_votes,
                COUNT(*) FILTER (WHERE votes.vote_up) AS positive_votes
            FROM
                votes
            WHERE
                votes.package_type = ",
        );
        builder.push_bind(PackageType::Snap);

        builder.push(match timeframe {
            Timeframe::Week => " AND votes.created >= NOW() - INTERVAL '1 week'",
            Timeframe::Month => " AND votes.created >= NOW() - INTERVAL '1 month'",
            Timeframe::Unspecified => "",
        });

        if let Some(category) = category {
            builder
                .push(
                    r"
                    AND votes.app_id IN (
                    SELECT snap_categories.snap_id FROM snap_categories 
                    WHERE snap_categories.category = ",
                )
                .push_bind(category)
                .push(")");
        }

        builder.push(" GROUP BY votes.package_type, votes.app_id");
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
                votes.app_id,
                votes.package_type,
                COUNT(*) AS total_votes,
                COUNT(*) FILTER (WHERE votes.vote_up) AS positive_votes
            FROM
                votes
            WHERE
                votes.package_type = $1
            AND
                votes.app_id = $2
            GROUP BY votes.package_type, votes.app_id
        "#,
    )
    .bind(PackageType::Snap)
    .bind(snap_id)
    .fetch_optional(conn)
    .await?;

    let summary = result.unwrap_or_else(|| VoteSummary {
        app_id: snap_id.to_string(),
        package_type: PackageType::Snap,
        total_votes: 0,
        positive_votes: 0,
    });

    Ok(summary)
}
