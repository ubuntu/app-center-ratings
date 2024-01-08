use crate::{
    app::AppContext,
    features::{
        chart::{
            entities::Chart, errors::ChartError, infrastructure::get_votes_summary_by_timeframe,
        },
        pb::chart::Timeframe,
    },
};
use tracing::error;

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
