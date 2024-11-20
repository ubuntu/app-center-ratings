//! Infrastructure definitions for the ratings center backend

use crate::{
    app::AppContext,
    features::{common::entities::VoteSummary, rating::errors::AppError},
};
use tracing::error;

/// Retrieves votes for the snap indicated by `snap_id` for the given [`AppContext`].
///
/// See the documentation for the common caller, [`get_rating`], for more information.
///
/// [`get_rating`]: crate::features::app::use_cases::get_rating
pub async fn get_votes_by_snap_id(
    app_ctx: &AppContext,
    snap_id: &str,
) -> Result<VoteSummary, AppError> {
    let mut pool = app_ctx
        .infrastructure()
        .repository()
        .await
        .map_err(|error| {
            error!("{error:?}");
            AppError::FailedToGetRating
        })?;

    let result = sqlx::query_as::<_, VoteSummary>(
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
    .fetch_optional(&mut *pool)
    .await
    .map_err(|error| {
        error!("{error:?}");
        AppError::Unknown
    })?;

    let summary = result.unwrap_or_else(|| VoteSummary {
        snap_id: snap_id.to_string(),
        total_votes: 0,
        positive_votes: 0,
    });

    Ok(summary)
}
