use tonic::{Request, Response, Status};

use self::pb::{GetVotesRequest, GetVotesResponse};

pub mod pb {
    tonic::include_proto!("ratings.feature.app");
}

pub use pb::app_server;

#[derive(Default, Debug)]
pub struct AppService;

#[tonic::async_trait]
impl pb::app_server::App for AppService {
    #[tracing::instrument]
    async fn get_votes(
        &self,
        request: Request<GetVotesRequest>,
    ) -> Result<Response<GetVotesResponse>, Status> {
        let app = request.into_inner().app;

        let payload = GetVotesResponse {
            app,
            total_up_votes: 10,
            total_down_votes: 3,
        };

        Ok(Response::new(payload))
    }
}
