use tonic::{Request, Response, Status};

use self::pb::{CastVoteRequest, ListMyVotesRequest, ListMyVotesResponse};

pub mod pb {
    tonic::include_proto!("ratings.feature.user");
}

pub use pb::user_server;

#[derive(Default, Debug)]
pub struct UserService;

#[tonic::async_trait]
impl pb::user_server::User for UserService {
    #[tracing::instrument]
    async fn cast_vote(&self, request: Request<CastVoteRequest>) -> Result<Response<()>, Status> {
        tracing::info!("");
        let request = request.into_inner();
        Ok(Response::new(()))
    }

    #[tracing::instrument]
    async fn list_my_votes(
        &self,
        request: Request<ListMyVotesRequest>,
    ) -> Result<Response<ListMyVotesResponse>, Status> {
        tracing::info!("");
        let request = request.into_inner();
        let payload = ListMyVotesResponse {
            app: "signal".to_string(),
            revision: "123".to_string(),
            vote: 2,
            ts: 11111111,
        };

        Ok(Response::new(payload))
    }
}
