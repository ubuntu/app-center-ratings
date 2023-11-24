use crate::{app::AppContext, features::common::entities::Vote};
use sqlx::Row;
use tracing::error;

use super::errors::AppError;

pub(crate) async fn get_votes_by_snap_id(
    app_ctx: &AppContext,
    snap_id: &str,
) -> Result<Vec<Vote>, AppError> {
    let mut pool = app_ctx
        .infrastructure()
        .repository()
        .await
        .map_err(|error| {
            error!("{error:?}");
            AppError::FailedToGetRating
        })?;
    let result = sqlx::query(
        r#"
                SELECT
                    votes.id,
                    votes.snap_id,
                    votes.vote_up
                FROM
                    votes
                WHERE
                    votes.snap_id = $1
            "#,
    )
    .bind(snap_id)
    .fetch_all(&mut *pool)
    .await
    .map_err(|error| {
        error!("{error:?}");
        AppError::Unknown
    })?;

    let votes: Vec<Vote> = result
        .into_iter()
        .map(|row| Vote {
            vote_up: row.get("vote_up"),
        })
        .collect();

    Ok(votes)
}
