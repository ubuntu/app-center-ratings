//! Definitions for getting vote summaries for the [`Chart`] implementations
//!
//! [`Chart`]: crate::features::chart::entities::Chart
use crate::{
    app::AppContext,
    features::{
        chart::errors::ChartError,
        common::entities::VoteSummary,
        pb::chart::{Category, Timeframe},
    },
};
use sqlx::QueryBuilder;
use tracing::error;

/// Retrieves the vote summary in the given [`AppContext`] over a given [`Timeframe`]
/// from the database.
pub(crate) async fn get_votes_summary(
    app_ctx: &AppContext,
    timeframe: Timeframe,
    category: Option<Category>,
) -> Result<Vec<VoteSummary>, ChartError> {
    let mut pool = app_ctx
        .infrastructure()
        .repository()
        .await
        .map_err(|error| {
            error!("{error:?}");
            ChartError::FailedToGetChart
        })?;

    let mut builder = QueryBuilder::new(
        r#"
    SELECT
        votes.snap_id,
        COUNT(*) AS total_votes,
        COUNT(*) FILTER (WHERE votes.vote_up) AS positive_votes
    FROM
        votes"#,
    );

    builder.push(match timeframe {
        Timeframe::Week => " WHERE votes.created >= NOW() - INTERVAL '1 week'",
        Timeframe::Month => " WHERE votes.created >= NOW() - INTERVAL '1 month'",
        Timeframe::Unspecified => "", // Adjust as needed for Unspecified case
    });

    if let Some(category) = category {
        builder
            .push(
                r#" 
                WHERE votes.snap_id IN (
                    SELECT snap_categories.snap_id FROM snap_categories 
                    WHERE snap_categories.category = "#,
            )
            .push_bind(category.to_kebab_case())
            .push(")");
    }

    builder.push(" GROUP BY votes.snap_id");

    let result = builder
        .build_query_as()
        .fetch_all(&mut *pool)
        .await
        .map_err(|error| {
            error!("{error:?}");
            ChartError::NotFound
        })?;

    Ok(result)
}
