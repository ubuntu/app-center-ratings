use crate::{app::AppContext, features::common::entities::VoteSummary};
use tracing::error;

use super::errors::AppError;

pub(crate) async fn get_votes_by_snap_id(
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
    .fetch_one(&mut *pool)
    .await
    .map_err(|error| {
        error!("{error:?}");
        AppError::Unknown
    })?;

    Ok(result)
}
