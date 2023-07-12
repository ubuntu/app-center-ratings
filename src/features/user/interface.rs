use tonic::{Request, Response, Status};

pub use protobuf::user_server;

use crate::features::user::use_cases;
use crate::utils::infrastructure::INFRA;

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

        let LoginRequest { uid } = request.into_inner();

        match use_cases::create_user(&uid).await {
            Ok(uid) => {
                let infra = INFRA.get().expect("INFRA should be initialised");
                let token = infra.jwt.encode(uid.to_string()).unwrap();

                let payload = LoginResponse { token };
                let response = Response::new(payload);

                Ok(response)
            }
            Err(error) => {
                tracing::error!("{error:?}");

                Err(Status::invalid_argument("uid"))
            }
        }
    }

    #[tracing::instrument]
    async fn delete_self(&self, _: Request<()>) -> Result<Response<()>, Status> {
        tracing::info!("delete self");
        Ok(Response::new(()))
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
