use crate::proto::user::{
    user_server::{User, UserServer},
    AuthenticateRequest, AuthenticateResponse, GetSnapVotesRequest, GetSnapVotesResponse,
    ListMyVotesRequest, ListMyVotesResponse, VoteRequest, Vote,
};
use time::OffsetDateTime;
use tonic::{Request, Response, Status};
use tracing::warn;

use ratings::{
    app::AppContext,
    features::user::{
        entities::{User as OldUser, Vote as OldVote},
        infrastructure::{
            create_or_seen_user, delete_user_by_client_hash, find_user_votes, save_vote_to_db,
            update_category, get_snap_votes_by_client_hash,
        },
    },
    utils::jwt::Claims,
};

// FIXME:
// Temporary while we finalize entites and db layer
impl Into<Vote> for OldVote {
    fn into(self) -> Vote{
        let timestamp = Some(prost_types::Timestamp {
            seconds: self.timestamp.unix_timestamp(),
            nanos: 0,
        });

        Vote {
            snap_id: self.snap_id,
            snap_revision: self.snap_revision as i32,
            vote_up: self.vote_up,
            timestamp,
        }
    }
}

#[tonic::async_trait]
impl User for UserService {
    #[tracing::instrument(level = "debug")]
    async fn authenticate(
        &self,
        request: Request<AuthenticateRequest>,
    ) -> Result<Response<AuthenticateResponse>, Status> {
        let app_ctx = request.extensions().get::<AppContext>().unwrap().clone();
        let AuthenticateRequest { id } = request.into_inner();

        if id.len() != EXPECTED_CLIENT_HASH_LENGTH {
            let error = format!(
                "Client hash must be of length {:?}",
                EXPECTED_CLIENT_HASH_LENGTH,
            );
            return Err(Status::invalid_argument(error));
        }

        let user = OldUser::new(&id);

        match create_or_seen_user(&app_ctx, user).await {
            Ok(user) => app_ctx
                .infrastructure()
                .jwt_encoder
                .encode(user.client_hash)
                .map(|token| AuthenticateResponse { token })
                .map(Response::new)
                .map_err(|_| Status::internal("internal")),
            Err(_error) => Err(Status::invalid_argument("id")),
        }
    }

    #[tracing::instrument(level = "debug")]
    async fn delete(&self, request: Request<()>) -> Result<Response<()>, Status> {
        let app_ctx = request.extensions().get::<AppContext>().unwrap().clone();
        let Claims {
            sub: client_hash, ..
        } = claims(&request);

        match delete_user_by_client_hash(&app_ctx, &client_hash).await {
            Ok(_) => Ok(Response::new(())),
            Err(_error) => Err(Status::unknown("Internal server error")),
        }
    }

    #[tracing::instrument(level = "debug")]
    async fn vote(&self, request: Request<VoteRequest>) -> Result<Response<()>, Status> {
        let app_ctx = request.extensions().get::<AppContext>().unwrap().clone();
        let Claims {
            sub: client_hash, ..
        } = claims(&request);
        let request = request.into_inner();

        let vote = OldVote {
            client_hash,
            snap_id: request.snap_id,
            snap_revision: request.snap_revision as u32,
            vote_up: request.vote_up,
            timestamp: OffsetDateTime::now_utc(),
        };

        // Ignore but log warning, it's not fatal
        update_category(&app_ctx, &vote.snap_id)
            .await
            .inspect_err(|e| warn!("{}", e));

        match save_vote_to_db(&app_ctx, vote).await {
            Ok(_) => Ok(Response::new(())),
            Err(_error) => Err(Status::unknown("Internal server error")),
        }
    }

    #[tracing::instrument(level = "debug")]
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

        let result = find_user_votes(&app_ctx, client_hash, snap_id_filter).await;

        match result {
            Ok(votes) => {
                let votes = votes
                    .into_iter()
                    .map(|vote| vote.into())
                    .collect();
                let payload = ListMyVotesResponse { votes };
                Ok(Response::new(payload))
            }
            Err(_error) => Err(Status::unknown("Internal server error")),
        }
    }

    #[tracing::instrument(level = "debug")]
    async fn get_snap_votes(
        &self,
        request: Request<GetSnapVotesRequest>,
    ) -> Result<Response<GetSnapVotesResponse>, Status> {
        let app_ctx = request.extensions().get::<AppContext>().unwrap().clone();
        let Claims {
            sub: client_hash, ..
        } = claims(&request);

        let GetSnapVotesRequest { snap_id } = request.into_inner();

        update_category(&app_ctx, &snap_id)
            .await
            .inspect_err(|e| warn!("{}", e));
        let result = get_snap_votes_by_client_hash(&app_ctx, snap_id, client_hash).await;

        match result {
            Ok(votes) => {
                let votes = votes
                    .into_iter()
                    .map(|vote| vote.into())
                    .collect();
                let payload = GetSnapVotesResponse { votes };
                Ok(Response::new(payload))
            }
            Err(_error) => Err(Status::unknown("Internal server error")),
        }
    }
}

/// Converts a request into a [`Claims`] value.
fn claims<T>(request: &Request<T>) -> Claims {
    request
        .extensions()
        .get::<Claims>()
        .expect("expected request to have claims")
        .clone()
}

/// The length we expect a client hash to be, in bytes
pub const EXPECTED_CLIENT_HASH_LENGTH: usize = 64;

/// An empty struct used to construct a [`UserServer`]
#[derive(Copy, Clone, Debug, Default)]
pub struct UserService;

impl UserService {
    /// The paths which are accessible without authentication, if any
    pub const PUBLIC_PATHS: [&'static str; 2] = [
        "ratings.features.user.User/Register",
        "ratings.features.user.User/Authenticate",
    ];

    /// Converts this service into its corresponding server
    pub fn to_server(self) -> UserServer<UserService> {
        self.into()
    }
}

impl From<UserService> for UserServer<UserService> {
    fn from(value: UserService) -> Self {
        UserServer::new(value)
    }
}
