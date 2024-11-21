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
};
use tonic::{Request, Response, Status};
use tracing::error;

/// The general service governing retrieving ratings for the store app.
#[derive(Clone)]
pub struct RatingService;

impl RatingService {
    /// The paths which are accessible without authentication, if any
    pub const PUBLIC_PATHS: [&'static str; 0] = [];

    /// Converts this service into its corresponding server
    pub fn to_server(self) -> AppServer<RatingService> {
        AppServer::new(self)
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

                Ok(Response::new(GetRatingResponse {
                    rating: Some(PbRating {
                        snap_id,
                        total_votes,
                        ratings_band: ratings_band as i32,
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
