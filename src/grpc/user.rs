use crate::{
    conn,
    db::{User, Vote},
    jwt::Claims,
    proto::user::{
        user_server::{self, UserServer},
        AuthenticateRequest, AuthenticateResponse, GetSnapVotesRequest, GetSnapVotesResponse,
        Vote as PbVote, VoteRequest,
    },
    ratings::{get_snap_name, update_categories, Error},
    Context,
};
use futures::future::try_join_all;
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
    pub fn new_server(ctx: Arc<Context>) -> UserServer<UserService> {
        UserServer::new(Self { ctx })
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
            Ok(user) => match self.ctx.jwt_encoder.encode(user.client_hash) {
                Ok(token) => Ok(Response::new(AuthenticateResponse { token })),
                Err(_) => Err(Status::internal("internal error")),
            },

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
                let votes = try_join_all(votes.into_iter().map(|vote| async {
                    let snap_name = get_snap_name(
                        &vote.snap_id,
                        &self.ctx.config.snapcraft_io_uri,
                        &self.ctx.http_client,
                    )
                    .await?;

                    Result::<PbVote, Error>::Ok(PbVote::from_vote_and_snap_name(vote, &snap_name))
                }))
                .await
                .map_err(|_| Status::unknown("Internal server error"))?;
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

impl PbVote {
    fn from_vote_and_snap_name(value: Vote, snap_name: &str) -> Self {
        let timestamp = Some(prost_types::Timestamp {
            seconds: value.timestamp.unix_timestamp(),
            nanos: value.timestamp.nanosecond() as i32,
        });

        Self {
            snap_id: value.snap_id,
            snap_revision: value.snap_revision as i32,
            vote_up: value.vote_up,
            timestamp,
            snap_name: snap_name.into(),
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
