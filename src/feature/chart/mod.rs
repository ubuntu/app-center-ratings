use tonic::{Request, Response, Status};

use self::pb::{ChartData, GetChartRequest, GetChartResponse};

pub mod pb {
    tonic::include_proto!("ratings.feature.chart");
}

pub use pb::chart_server;

#[derive(Default, Debug)]
pub struct ChartService;

#[tonic::async_trait]
impl pb::chart_server::Chart for ChartService {
    #[tracing::instrument]
    async fn get_chart(
        &self,
        request: Request<GetChartRequest>,
    ) -> Result<Response<GetChartResponse>, Status> {
        tracing::info!("Received request");
        let request = request.into_inner();

        let payload = GetChartResponse {
            timeframe: request.timeframe,
            r#type: request.r#type,
            ordered_chart_data: vec![ChartData {
                app: String::from("signal"),
                total_up_votes: 10,
                total_down_votes: 2,
            }],
        };

        Ok(Response::new(payload))
    }
}
