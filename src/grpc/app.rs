use std::sync::Arc;

use crate::{
    conn,
    db::VoteSummary,
    proto::{
        app::{
            app_server::{App, AppServer},
            GetRatingRequest, GetRatingResponse,
        },
        common::Rating as PbRating,
    },
    ratings::Rating,
    Context,
};
use tonic::{Request, Response, Status};
use tracing::error;

/// The general service governing retrieving ratings for the store app.
#[derive(Clone)]
pub struct RatingService {
    ctx: Arc<Context>,
}

impl RatingService {
    pub fn new_server(ctx: Arc<Context>) -> AppServer<RatingService> {
        AppServer::new(Self { ctx })
    }
}

#[tonic::async_trait]
impl App for RatingService {
    async fn get_rating(
        &self,
        request: Request<GetRatingRequest>,
    ) -> Result<tonic::Response<GetRatingResponse>, Status> {
        let GetRatingRequest { snap_id } = request.into_inner();
        if snap_id.is_empty() {
            return Err(Status::invalid_argument("snap id"));
        }

        match VoteSummary::get_by_snap_id(&snap_id, conn!()).await {
            Ok(votes) => {
                let Rating {
                    snap_id,
                    total_votes,
                    ratings_band,
                    snap_name,
                } = Rating::new(votes, &self.ctx)
                    .await
                    .map_err(|_| Status::unknown("Internal server error"))?;

                Ok(Response::new(GetRatingResponse {
                    rating: Some(PbRating {
                        snap_id,
                        total_votes,
                        ratings_band: ratings_band as i32,
                        snap_name,
                    }),
                }))
            }

            Err(e) => {
                error!("Error calling get_votes_by_snap_id: {:?}", e);
                Err(Status::unknown("Internal server error"))
            }
        }
    }
}
