//! Definitions and utilities for building the [`ChartService`] for using the [`Chart`] feature.
//!
//! [`Chart`]: crate::features::chart::entities::Chart
use ratings::features::chart::{errors::ChartError, entities::Chart as OldChart};
use crate::proto::chart::{chart_server::{ChartServer, Chart}, GetChartRequest, GetChartResponse};

use tonic::{Request, Response, Status};
use tracing::error;

use ratings::{
    app::AppContext,
    features::chart::infrastructure::get_votes_summary,
    features::pb::chart::Category,
    features::pb::chart::Timeframe,
};

/// An empty struct denoting that allows the building of a [`ChartServer`].
#[derive(Copy, Clone, Debug, Default)]
pub struct ChartService;

impl ChartService {
    /// The paths which are accessible without authentication, if any
    pub const PUBLIC_PATHS: [&'static str; 0] = [];

    /// Converts this service into its corresponding server
    pub fn to_server(self) -> ChartServer<ChartService> {
        self.into()
    }
}

impl From<ChartService> for ChartServer<ChartService> {
    fn from(value: ChartService) -> Self {
        ChartServer::new(value)
    }
}

#[tonic::async_trait]
impl Chart for ChartService {
    #[tracing::instrument]
    async fn get_chart(
        &self,
        request: Request<GetChartRequest>,
    ) -> Result<Response<GetChartResponse>, Status> {
        let app_ctx = request.extensions().get::<AppContext>().unwrap().clone();

        let GetChartRequest {
            timeframe,
            category,
        } = request.into_inner();

        let category = match category {
            Some(category) => Some(
                Category::try_from(category)
                    .map_err(|_| Status::invalid_argument("invalid category value"))?,
            ),
            None => None,
        };

        let timeframe = Timeframe::try_from(timeframe).unwrap_or(Timeframe::Unspecified);

        let result = get_votes_summary(&app_ctx, timeframe, category).await.map_err(|error| {
            error!("{error:?}");
            ChartError::Unknown
        });

        match result {
            Ok(c) => {
                let chart = OldChart::new(timeframe, c);
                let ordered_chart_data = chart 
                    .chart_data
                    .into_iter()
                    .map(|chart_data| chart_data.into_protobuf_chart_data())
                    .collect();

                let payload = GetChartResponse {
                    timeframe: timeframe.into(),
                    category: category.map(|v| v.into()),
                    ordered_chart_data,
                };
                Ok(Response::new(payload))
            }
            Err(error) => match error {
                ChartError::NotFound => {
                    Err(Status::not_found("Cannot find data for given timeframe."))
                }
                _ => Err(Status::unknown("Internal server error")),
            },
        }
    }
}
