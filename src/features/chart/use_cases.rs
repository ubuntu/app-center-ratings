//! Utility functions for using the [`Chart`] feature in one call.

use crate::{
    app::AppContext,
    features::{
        chart::{entities::Chart, errors::ChartError, infrastructure::get_votes_summary},
        pb::chart::{Category, Timeframe},
    },
};
use tracing::error;

/// Gets a chart over the given [`Timeframe`] within the given [`AppContext`]. Either ends up returning
/// a [`Chart`] or else one of the many [`ChartError`]s in case the timeframe is invalid or another database error
/// happens.
pub async fn get_chart(
    app_ctx: &AppContext,
    timeframe: Timeframe,
    category: Option<Category>,
) -> Result<Chart, ChartError> {
    let votes = get_votes_summary(app_ctx, timeframe, category)
        .await
        .map_err(|error| {
            error!("{error:?}");
            ChartError::Unknown
        })?;

    let chart = Chart::new(timeframe, votes);

    Ok(chart)
}
