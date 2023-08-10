use time::OffsetDateTime;

use tonic::{Request, Response, Status};

pub use protobuf::user_server;

use crate::app::AppContext;
use crate::utils::jwt::Claims;

use super::entities::Vote;
use super::service::UserService;
use super::use_cases;

use self::protobuf::{
    AuthenticateRequest, AuthenticateResponse, ListMyVotesRequest, ListMyVotesResponse,
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
        let app_ctx = request.extensions().get::<AppContext>().unwrap().clone();
        let RegisterRequest { id } = request.into_inner();

        if id.len() != EXPECTED_CLIENT_HASH_LENGTH {
            return Err(Status::invalid_argument("id"));
        }

        match use_cases::register(&app_ctx, &id).await {
            Ok(user) => app_ctx
                .infrastructure()
                .jwt
                .encode(user.client_hash)
                .map(|token| RegisterResponse { token })
                .map(Response::new)
                .map_err(|_| Status::internal("internal")),
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
        let app_ctx = request.extensions().get::<AppContext>().unwrap().clone();
        let AuthenticateRequest { id } = request.into_inner();

        if id.len() != EXPECTED_CLIENT_HASH_LENGTH {
            return Err(Status::invalid_argument("id"));
        }

        match use_cases::authenticate(&app_ctx, &id).await {
            Ok(exists) => {
                if exists {
                    app_ctx
                        .infrastructure()
                        .jwt
                        .encode(id)
                        .map(|token| AuthenticateResponse { token })
                        .map(Response::new)
                        .map_err(|_| Status::internal("internal"))
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
        let app_ctx = request.extensions().get::<AppContext>().unwrap().clone();
        let Claims {
            sub: client_hash, ..
        } = claims(&request);

        match use_cases::delete_user(&app_ctx, &client_hash).await {
            Ok(_) => Ok(Response::new(())),
            Err(error) => {
                tracing::error!("{error:?}");
                Err(Status::unknown("Internal server error"))
            }
        }
    }

    #[tracing::instrument]
    async fn vote(&self, request: Request<VoteRequest>) -> Result<Response<()>, Status> {
        let app_ctx = request.extensions().get::<AppContext>().unwrap().clone();
        let Claims {
            sub: client_hash, ..
        } = claims(&request);
        let request = request.into_inner();

        let vote = Vote {
            client_hash,
            snap_id: request.snap_id,
            snap_revision: request.snap_revision as u32,
            vote_up: request.vote_up,
            timestamp: OffsetDateTime::now_utc(),
        };

        match use_cases::vote(&app_ctx, vote).await {
            Ok(_) => Ok(Response::new(())),
            Err(error) => {
                tracing::error!("{error:?}");
                Err(Status::unknown("Internal server error"))
            }
        }
    }

    #[tracing::instrument]
    async fn list_my_votes(
        &self,
        request: Request<ListMyVotesRequest>,
    ) -> Result<Response<ListMyVotesResponse>, Status> {
        let app_ctx = request.extensions().get::<AppContext>().unwrap().clone();
        let Claims {
            sub: client_hash, ..
        } = claims(&request);
        let ListMyVotesRequest { snap_id_filter } = request.into_inner();
        let snap_id_filter = if snap_id_filter.is_empty() {
            None
        } else {
            Some(snap_id_filter)
        };
        let result = use_cases::list_my_votes(&app_ctx, client_hash, snap_id_filter).await;

        match result {
            Ok(votes) => {
                let votes = votes.into_iter().map(|vote| vote.into_dto()).collect();
                let payload = ListMyVotesResponse { votes };
                Ok(Response::new(payload))
            }
            Err(error) => {
                tracing::error!("{error:?}");
                Err(Status::unknown("Internal server error"))
            }
        }
    }
}

fn claims<T>(request: &Request<T>) -> Claims {
    request
        .extensions()
        .get::<Claims>()
        .expect("expected request to have claims")
        .clone()
}

pub const EXPECTED_CLIENT_HASH_LENGTH: usize = 64;
