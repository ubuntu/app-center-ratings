use tonic::{Request, Response, Status};

pub use protobuf::user_server;

use crate::utils::{infrastructure::INFRA, jwt::Claims};

use super::entities::Vote;
use super::service::UserService;
use super::use_cases;

use self::protobuf::{
    AuthenticateRequest, AuthenticateResponse, ListVotesRequest, ListVotesResponse,
    RegisterRequest, RegisterResponse, User, VoteRequest,
};

pub mod protobuf {
    pub use self::user_server::User;

    tonic::include_proto!("ratings.features.user");
}

#[tonic::async_trait]
impl User for UserService {
    #[tracing::instrument]
    async fn register(
        &self,
        request: Request<RegisterRequest>,
    ) -> Result<Response<RegisterResponse>, Status> {
        let RegisterRequest { id } = request.into_inner();

        if !validate_client_hash(&id) {
            return Err(Status::invalid_argument("id"));
        }

        match use_cases::register(&id).await {
            Ok(user) => INFRA
                .get()
                .expect("INFRA should be initialised")
                .jwt
                .encode(user.client_hash)
                .map(|token| RegisterResponse { token })
                .map(|payload| Response::new(payload))
                .map_err(|error| Status::internal("internal")),
            Err(error) => {
                tracing::error!("{error:?}");
                Err(Status::invalid_argument("id"))
            }
        }
    }

    #[tracing::instrument]
    async fn authenticate(
        &self,
        request: Request<AuthenticateRequest>,
    ) -> Result<Response<AuthenticateResponse>, Status> {
        let AuthenticateRequest { id } = request.into_inner();

        if !validate_client_hash(&id) {
            return Err(Status::invalid_argument("id"));
        }

        match use_cases::authenticate(&id).await {
            Ok(exists) => {
                if exists {
                    INFRA
                        .get()
                        .expect("INFRA should be initialised")
                        .jwt
                        .encode(id)
                        .map(|token| AuthenticateResponse { token })
                        .map(|payload| Response::new(payload))
                        .map_err(|error| Status::internal("internal"))
                } else {
                    tracing::info!("no record for client hash {id}");
                    Err(Status::unauthenticated("invalid credentials"))
                }
            }
            Err(error) => {
                tracing::error!("{error:?}");
                Err(Status::invalid_argument("id"))
            }
        }
    }

    #[tracing::instrument]
    async fn delete(&self, request: Request<()>) -> Result<Response<()>, Status> {
        let Claims {
            sub: client_hash, ..
        } = get_claims(&request);

        match use_cases::delete_user(&client_hash).await {
            Ok(_) => Ok(Response::new(())),
            Err(error) => {
                tracing::error!("{error:?}");
                Ok(Response::new(()))
            }
        }
    }

    #[tracing::instrument]
    async fn vote(&self, request: Request<VoteRequest>) -> Result<Response<()>, Status> {
        let Claims {
            sub: client_hash, ..
        } = get_claims(&request);
        let request = request.into_inner();

        let vote = Vote {
            client_hash,
            snap_id: request.snap_id,
            snap_revision: request.snap_revision as u32,
            vote_up: request.vote_up,
        };

        match use_cases::vote(vote).await {
            Ok(_) => Ok(Response::new(())),
            Err(error) => {
                tracing::error!("{error:?}");
                Ok(Response::new(()))
            }
        }
    }

    #[tracing::instrument]
    async fn list_votes(
        &self,
        request: Request<ListVotesRequest>,
    ) -> Result<Response<ListVotesResponse>, Status> {
        todo!()
    }
}

fn get_claims<T>(request: &Request<T>) -> Claims {
    request
        .extensions()
        .get::<Claims>()
        .expect("expected request to have claims")
        .clone()
}

pub const EXPECTED_CLIENT_HASH_LENGTH: usize = 64;

fn validate_client_hash(value: &str) -> bool {
    value.len() == EXPECTED_CLIENT_HASH_LENGTH
}
