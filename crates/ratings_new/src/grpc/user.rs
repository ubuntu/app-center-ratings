use crate::{
    conn,
    context::Claims,
    db::{User, Vote},
    proto::user::{
        user_server::{self, UserServer},
        AuthenticateRequest, AuthenticateResponse, GetSnapVotesRequest, GetSnapVotesResponse,
        ListMyVotesRequest, ListMyVotesResponse, Vote as PbVote, VoteRequest,
    },
    ratings::update_categories,
    Context,
};
use std::sync::Arc;
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

impl UserService {
    /// The paths which are accessible without authentication, if any
    pub const PUBLIC_PATHS: [&'static str; 2] = [
        "ratings.features.user.User/Register",
        "ratings.features.user.User/Authenticate",
    ];

    /// Converts this service into its corresponding server
    pub fn to_server(self) -> UserServer<UserService> {
        UserServer::new(self)
    }
}

#[tonic::async_trait]
impl user_server::User for UserService {
    async fn authenticate(
        &self,
        request: Request<AuthenticateRequest>,
    ) -> Result<Response<AuthenticateResponse>, Status> {
        let AuthenticateRequest { id } = request.into_inner();
        if id.len() != EXPECTED_CLIENT_HASH_LENGTH {
            let error = format!(
                "Client hash must be of length {:?}",
                EXPECTED_CLIENT_HASH_LENGTH,
            );
            return Err(Status::invalid_argument(error));
        }

        match User::create_or_seen(&id, conn!()).await {
            Ok(user) => {
                let token = match self.ctx.jwt_encoder.encode(user.client_hash) {
                    Ok(token) => token,
                    Err(_) => return Err(Status::internal("internal error")),
                };

                Ok(Response::new(AuthenticateResponse { token }))
            }

            Err(_error) => Err(Status::invalid_argument("id")),
        }
    }

    async fn delete(&self, mut request: Request<()>) -> Result<Response<()>, Status> {
        let Claims {
            sub: client_hash, ..
        } = claims(&mut request);

        match User::delete_by_client_hash(&client_hash, conn!()).await {
            Ok(_) => Ok(Response::new(())),

            Err(e) => {
                error!("Error in delete_user_by_client_hash: {:?}", e);
                Err(Status::unknown("Internal server error"))
            }
        }
    }

    async fn vote(&self, mut request: Request<VoteRequest>) -> Result<Response<()>, Status> {
        let Claims { sub, .. } = claims(&mut request);
        let VoteRequest {
            snap_id,
            snap_revision,
            vote_up,
        } = request.into_inner();
        let conn = conn!();

        // Ignore but log warning, it's not fatal
        if let Err(e) = update_categories(&snap_id, &self.ctx, conn).await {
            warn!("unable to update categories for snap: {e}");
        }

        let vote = Vote {
            client_hash: sub,
            snap_id,
            snap_revision: snap_revision as u32,
            vote_up,
            timestamp: OffsetDateTime::now_utc(),
        };

        match vote.save_to_db(conn).await {
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
        let Claims {
            sub: client_hash, ..
        } = claims(&mut request);

        let ListMyVotesRequest { snap_id_filter } = request.into_inner();
        let snap_id_filter = if snap_id_filter.is_empty() {
            None
        } else {
            Some(snap_id_filter)
        };

        match Vote::get_all_by_client_hash(&client_hash, snap_id_filter, conn!()).await {
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
        let Claims {
            sub: client_hash, ..
        } = claims(&mut request);
        let GetSnapVotesRequest { snap_id } = request.into_inner();

        let conn = conn!();

        // Ignore but log warning, it's not fatal
        if let Err(e) = update_categories(&snap_id, &self.ctx, conn).await {
            warn!("unable to update categories for snap: {e}");
        }

        match Vote::get_all_by_client_hash(&client_hash, Some(snap_id), conn).await {
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

impl From<Vote> for PbVote {
    fn from(value: Vote) -> Self {
        let timestamp = Some(prost_types::Timestamp {
            seconds: value.timestamp.unix_timestamp(),
            nanos: value.timestamp.nanosecond() as i32,
        });

        Self {
            snap_id: value.snap_id,
            snap_revision: value.snap_revision as i32,
            vote_up: value.vote_up,
            timestamp,
        }
    }
}

#[inline]
fn claims<T>(request: &mut Request<T>) -> Claims {
    request
        .extensions_mut()
        .remove::<Claims>()
        .expect("expected request to have claims")
}
