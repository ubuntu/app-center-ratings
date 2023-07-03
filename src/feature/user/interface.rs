use self::protobuf::{CastVoteRequest, ListMyVotesRequest, ListMyVotesResponse, User};
pub use protobuf::user_server;
use tonic::{Request, Response, Status};
use crate::app::Context;

use super::service::UserService;

pub mod protobuf {
    pub use self::user_server::{User, UserServer};
    tonic::include_proto!("ratings.feature.user");
}

#[tonic::async_trait]
impl User for UserService {
    #[tracing::instrument]
    async fn delete_self(&self, request: Request<()>) -> Result<Response<()>, Status> {
        tracing::info!("delete self");
        Ok(Response::new(()))
    }

    #[tracing::instrument]
    async fn cast_vote(&self, request: Request<CastVoteRequest>) -> Result<Response<()>, Status> {
        todo!()
    }

    #[tracing::instrument]
    async fn list_my_votes(
        &self,
        request: Request<ListMyVotesRequest>,
    ) -> Result<Response<ListMyVotesResponse>, Status> {
        todo!()
    }
}
