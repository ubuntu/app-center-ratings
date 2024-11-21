use std::sync::Arc;

use crate::{
    conn,
    context::Claims,
    // FIXME: DbVote? DbUser?
    db::user::User as DbUser,
    db::vote::Vote as DbVote,
    proto::user::{
        user_server::{User, UserServer},
        AuthenticateRequest, AuthenticateResponse, GetSnapVotesRequest, GetSnapVotesResponse,
        ListMyVotesRequest, ListMyVotesResponse, Vote, VoteRequest,
    },
    ratings::{
        categories::update_categories,
        users::{create_or_seen_user, delete_user_by_client_hash},
        votes::{find_user_votes, get_snap_votes_by_client_hash, save_vote_to_db},
    },
    Context,
};

use time::OffsetDateTime;
use tonic::{Request, Response, Status};
use tracing::{error, warn};

/// The length we expect a client hash to be, in bytes
pub const EXPECTED_CLIENT_HASH_LENGTH: usize = 64;

/// An empty struct used to construct a [`UserServer`]
#[derive(Clone)]
pub struct UserService {
    ctx: Arc<Context>,
}
// Store jwt encoder here. If needed in multiple places, arc it.

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

#[tonic::async_trait]
impl User for UserService {
    async fn authenticate(
        &self,
        request: Request<AuthenticateRequest>,
    ) -> Result<Response<AuthenticateResponse>, Status> {
        // TODO: is there where we expect the pg_connection?
        let conn = conn!();
        let AuthenticateRequest { id } = request.into_inner();

        if id.len() != EXPECTED_CLIENT_HASH_LENGTH {
            let error = format!(
                "Client hash must be of length {:?}",
                EXPECTED_CLIENT_HASH_LENGTH,
            );
            return Err(Status::invalid_argument(error));
        }

        // FIXME: replace with new struct
        let user = DbUser::new(&id);

        match create_or_seen_user(user, &self.ctx, conn).await {
            Ok(user) => self
                .ctx
                .jwt_encoder
                .encode(user.client_hash)
                // Match on the encode, build the ok / error varients in there, out of a chain.
                .map(|token| AuthenticateResponse { token })
                .map(Response::new)
                .map_err(|_| Status::internal("internal")),
            Err(_error) => Err(Status::invalid_argument("id")),
        }
    }

    async fn delete(&self, mut request: Request<()>) -> Result<Response<()>, Status> {
        let ctx = request
            .extensions_mut()
            .remove::<Context>()
            .expect("Expected AppContext to be present");
        let conn = conn!();
        let Claims {
            sub: client_hash, ..
        } = claims(&mut request);

        match delete_user_by_client_hash(&client_hash, &ctx, conn).await {
            // FIXME (maybe?)
            // favor a into pattern if we do this in many places
            Ok(_) => Ok(Response::new(())),
            Err(e) => {
                error!("Error in delete_user_by_client_hash: {:?}", e);
                Err(Status::unknown("Internal server error"))
            }
        }
    }

    async fn vote(&self, mut request: Request<VoteRequest>) -> Result<Response<()>, Status> {
        let ctx = request
            .extensions_mut()
            .remove::<Context>()
            .expect("Expected AppContext to be present");
        let conn = conn!();
        let Claims {
            sub: client_hash, ..
        } = claims(&mut request);
        let request = request.into_inner();

        let vote = DbVote {
            client_hash,
            snap_id: request.snap_id,
            snap_revision: request.snap_revision as u32,
            vote_up: request.vote_up,
            timestamp: OffsetDateTime::now_utc(),
        };

        // Ignore but log warning, it's not fatal
        let _ = update_categories(&vote.snap_id, &ctx, conn)
            .await
            .inspect_err(|e| warn!("{}", e));

        match save_vote_to_db(&ctx, vote, conn).await {
            Ok(_) => Ok(Response::new(())),

            Err(e) => {
                error!("Error in save_vote_to_db: {:?}", e);
                Err(Status::unknown("Internal server error"))
            }
        }
    }

    async fn list_my_votes(
        &self,
        mut request: Request<ListMyVotesRequest>,
    ) -> Result<Response<ListMyVotesResponse>, Status> {
        let ctx = request
            .extensions_mut()
            .remove::<Context>()
            .expect("Expected AppContext to be present");
        let conn = conn!();
        let Claims {
            sub: client_hash, ..
        } = claims(&mut request);
        let ListMyVotesRequest { snap_id_filter } = request.into_inner();
        let snap_id_filter = if snap_id_filter.is_empty() {
            None
        } else {
            Some(snap_id_filter)
        };

        let result = find_user_votes(&ctx, client_hash, snap_id_filter, conn).await;

        match result {
            Ok(votes) => {
                let votes = votes.into_iter().map(Into::into).collect();
                let payload = ListMyVotesResponse { votes };

                Ok(Response::new(payload))
            }
            Err(e) => {
                error!("Error in find_user_votes: {:?}", e);
                Err(Status::unknown("Internal server error"))
            }
        }
    }

    async fn get_snap_votes(
        &self,
        mut request: Request<GetSnapVotesRequest>,
    ) -> Result<Response<GetSnapVotesResponse>, Status> {
        // FIXME: will turn into con macro
        let ctx = request
            .extensions_mut()
            .remove::<Context>()
            .expect("Expected AppContext to be present");
        let conn = conn!();
        let Claims {
            sub: client_hash, ..
        } = claims(&mut request);

        let GetSnapVotesRequest { snap_id } = request.into_inner();

        // Ignore but log warning, it's not fatal
        let _ = update_categories(&snap_id, &ctx, conn)
            .await
            .inspect_err(|e| warn!("{}", e));

        let result = get_snap_votes_by_client_hash(&ctx, snap_id, client_hash, conn).await;

        match result {
            Ok(votes) => {
                let votes = votes.into_iter().map(|vote| vote.into()).collect();
                let payload = GetSnapVotesResponse { votes };

                Ok(Response::new(payload))
            }
            Err(e) => {
                error!("Error in get_snap_votes_by_client_hash: {:?}", e);
                Err(Status::unknown("Internal server error"))
            }
        }
    }
}

impl From<DbVote> for Vote {
    fn from(value: DbVote) -> Vote {
        let timestamp = Some(prost_types::Timestamp {
            seconds: value.timestamp.unix_timestamp(),
            nanos: value.timestamp.nanosecond() as i32,
        });

        Vote {
            snap_id: value.snap_id,
            snap_revision: value.snap_revision as i32,
            vote_up: value.vote_up,
            timestamp,
        }
    }
}

/// Converts a request into a [`Claims`] value.
fn claims<T>(request: &mut Request<T>) -> Claims {
    request
        .extensions_mut()
        .remove::<Claims>()
        .expect("expected request to have claims")
}
