use std::future::Future;

use tonic::{Request, Response, Status};

pub use protobuf::user_server;

use crate::features::user::use_cases;
use crate::utils::infrastructure::INFRA;
use crate::utils::jwt::Claims;

use super::service::UserService;

use self::protobuf::{
    CastVoteRequest, ListMyVotesRequest, ListMyVotesResponse, LoginRequest, LoginResponse, User,
};

pub mod protobuf {
    pub use self::user_server::User;

    tonic::include_proto!("ratings.features.user");
}

#[tonic::async_trait]
impl User for UserService {
    #[tracing::instrument]
    async fn login(
        &self,
        request: Request<LoginRequest>,
    ) -> Result<Response<LoginResponse>, Status> {
        tracing::info!("register");

        let LoginRequest { user_id } = request.into_inner();

        match use_cases::create_user(&user_id).await {
            Ok(user_id) => {
                let infra = INFRA.get().expect("INFRA should be initialised");
                let token = infra.jwt.encode(user_id.to_string()).unwrap();

                let payload = LoginResponse { token };
                let response = Response::new(payload);

                Ok(response)
            }
            Err(error) => {
                tracing::error!("{error:?}");
                Err(Status::invalid_argument("user_id"))
            }
        }
    }

    #[tracing::instrument]
    async fn delete(&self, request: Request<()>) -> Result<Response<()>, Status> {
        tracing::info!("deleting self");
        let claim = request
            .extensions()
            .get::<Claims>()
            .expect("request should have claim");
        let user_id = claim.sub.clone();

        match use_cases::delete_user(&user_id).await {
            Ok(_) => Ok(Response::new(())),
            Err(error) => {
                tracing::error!("{error:?}");
                Ok(Response::new(()))
            }
        }
    }

    #[tracing::instrument]
    async fn cast_vote(&self, _: Request<CastVoteRequest>) -> Result<Response<()>, Status> {
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
