//! Contains generation and definitions for the [`AppService`]
use crate::proto::app::app_server::AppServer;
use crate::proto::common::Rating;

//replace
use ratings::{app::AppContext, features::common::entities::Rating as OldRating};
use ratings::features::rating::infrastructure::get_votes_by_snap_id;
use tracing::error;

use crate::proto::app::{app_server::App, GetRatingRequest, GetRatingResponse};
use tonic::{Request, Response, Status};

/// The general service governing retrieving ratings for the store app.
#[derive(Copy, Clone, Debug, Default)]
pub struct RatingService;

impl RatingService {
    /// The paths which are accessible without authentication, if any
    pub const PUBLIC_PATHS: [&'static str; 0] = [];

    /// Converts this service into its corresponding server
    pub fn to_server(self) -> AppServer<RatingService> {
        self.into()
    }
}

impl From<RatingService> for AppServer<RatingService> {
    fn from(value: RatingService) -> Self {
        AppServer::new(value)
    }
}

#[tonic::async_trait]
impl App for RatingService {
    #[tracing::instrument(level = "debug")]
    async fn get_rating(
        &self,
        request: Request<GetRatingRequest>,
    ) -> Result<tonic::Response<GetRatingResponse>, Status> {
        let app_ctx = request.extensions().get::<AppContext>().unwrap().clone();

        let GetRatingRequest { snap_id } = request.into_inner();

        if snap_id.is_empty() {
            return Err(Status::invalid_argument("snap id"));
        }

        // let result = use_cases::get_rating(&app_ctx, snap_id).await;
        
        match get_votes_by_snap_id(&app_ctx, &snap_id).await {
            Ok(votes) => {
                let rating = OldRating::new(votes);
                let payload = GetRatingResponse {
                    rating: Some(rating.into()),
                };
                Ok(Response::new(payload))
            }
            Err(e) => 
            {
                error!("Error calling get_votes_by_snap_id: {:?}", e);
                Err(Status::unknown("Internal server error")) },
        }
    }
}

impl From<OldRating> for Rating{
    fn from(value: OldRating) -> Rating{
        Rating {
            snap_id: value.snap_id,
            total_votes: value.total_votes,
            ratings_band: value.ratings_band as i32,
        }
    }
}
