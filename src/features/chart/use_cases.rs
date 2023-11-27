use super::{errors::ChartError, infrastructure::get_votes_summary_by_timeframe};
use crate::app::AppContext;
use crate::features::chart::entities::Chart;
use crate::features::pb::chart::Timeframe;
use tracing::error;
// use super::{entities::Rating, errors::AppError, infrastructure::get_votes_by_snap_id};

pub async fn get_chart(app_ctx: &AppContext, timeframe: Timeframe) -> Result<Chart, ChartError> {
    let votes = get_votes_summary_by_timeframe(app_ctx, timeframe)
        .await
        .map_err(|error| {
            error!("{error:?}");
            ChartError::Unknown
        })?;

    let chart = Chart::new(timeframe, votes);

    Ok(chart)
}
