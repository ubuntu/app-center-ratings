use crate::{
    app::AppContext,
    features::{chart::errors::ChartError, common::entities::VoteSummary, pb::chart::Timeframe},
};
use tracing::error;

pub(crate) async fn get_votes_summary_by_timeframe(
    app_ctx: &AppContext,
    timeframe: Timeframe,
) -> Result<Vec<VoteSummary>, ChartError> {
    let mut pool = app_ctx
        .infrastructure()
        .repository()
        .await
        .map_err(|error| {
            error!("{error:?}");
            ChartError::FailedToGetChart
        })?;

    // Generate WHERE clause based on timeframe
    let where_clause = match timeframe {
        Timeframe::Week => "WHERE votes.created >= NOW() - INTERVAL '1 week'",
        Timeframe::Month => "WHERE votes.created >= NOW() - INTERVAL '1 month'",
        Timeframe::Unspecified => "", // Adjust as needed for Unspecified case
    };

    let query = format!(
        r#"
            SELECT
                votes.snap_id,
                COUNT(*) AS total_votes,
                COUNT(*) FILTER (WHERE votes.vote_up) AS positive_votes
            FROM
                votes
            {}
            GROUP BY votes.snap_id
        "#,
        where_clause
    );

    let result = sqlx::query_as::<_, VoteSummary>(&query)
        .fetch_all(&mut *pool)
        .await
        .map_err(|error| {
            error!("{error:?}");
            ChartError::NotFound
        })?;

    Ok(result)
}
