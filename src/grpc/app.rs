use crate::{
    conn,
    db::VoteSummary,
    proto::{
        app::{
            app_server::{App, AppServer},
            GetBulkRatingsRequest, GetBulkRatingsResponse, GetRatingRequest, GetRatingResponse,
        },
        common::Rating as PbRating,
    },
    ratings::{get_snap_name, Rating},
    Context,
};
use std::{error::Error, sync::Arc};
use tonic::{Request, Response, Status};
use tracing::error;

/// The general service governing retrieving ratings for the store app.
#[derive(Clone)]
pub struct RatingService {
    ctx: Arc<Context>,
}

impl RatingService {
    pub fn new_server(ctx: Arc<Context>) -> AppServer<RatingService> {
        AppServer::new(RatingService { ctx })
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
                } = Rating::from(votes);

                let snap_name = get_snap_name(
                    &snap_id,
                    &self.ctx.config.snapcraft_io_uri,
                    &self.ctx.http_client,
                )
                .await
                .map_err(|e| {
                    let mut err = &e as &dyn Error;
                    let mut error = format!("{err}");
                    while let Some(src) = err.source() {
                        error.push_str(&format!("\n\nCaused by: {src}"));
                        err = src;
                    }
                    error!(%error, "unable to fetch snap name");
                    Status::unknown("Internal server error")
                })?;

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
    async fn get_bulk_ratings(
        &self,
        request: Request<GetBulkRatingsRequest>,
    ) -> Result<tonic::Response<GetBulkRatingsResponse>, Status> {
        Err(Status::unimplemented("get_bulk_ratings is not implemented"))
    }
}
