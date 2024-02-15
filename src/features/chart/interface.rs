//! Contains trait implementations for the chart feature.
use crate::{
    app::AppContext,
    features::{
        chart::{errors::ChartError, service::ChartService, use_cases},
        pb::chart::{chart_server::Chart, GetChartRequest, GetChartResponse, Timeframe},
    },
};
use tonic::{Request, Response, Status};

#[tonic::async_trait]
impl Chart for ChartService {
    #[tracing::instrument]
    async fn get_chart(
        &self,
        request: Request<GetChartRequest>,
    ) -> Result<Response<GetChartResponse>, Status> {
        let app_ctx = request.extensions().get::<AppContext>().unwrap().clone();

        let GetChartRequest { timeframe, .. } = request.into_inner();

        let timeframe = match timeframe {
            0 => Timeframe::Unspecified,
            1 => Timeframe::Week,
            2 => Timeframe::Month,
            _ => Timeframe::Unspecified,
        };

        let result = use_cases::get_chart(&app_ctx, timeframe).await;

        match result {
            Ok(result) => {
                let ordered_chart_data = result
                    .chart_data
                    .into_iter()
                    .map(|chart_data| chart_data.into_protobuf_chart_data())
                    .collect();

                let payload = GetChartResponse {
                    timeframe: timeframe.into(),
                    category: None,
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
