//! Contains generation and definitions for the [`AppService`]

// FIXME: Remove these dependencies
use ratings::features::common::entities::Rating as OldRating;

use crate::proto::common::Rating;
use crate::ratings::votes::get_votes_by_snap_id;
use crate::{proto::app::app_server::AppServer, Context};

use sqlx::PgConnection;
//replace
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
        mut request: Request<GetRatingRequest>,
    ) -> Result<tonic::Response<GetRatingResponse>, Status> {
        let ctx = request
            .extensions_mut()
            .remove::<Context>()
            .expect("Expected Context to be present");
        let mut conn = request
            .extensions_mut()
            .remove::<PgConnection>()
            .expect("Expected PgConnection to be present");

        let GetRatingRequest { snap_id } = request.into_inner();

        if snap_id.is_empty() {
            return Err(Status::invalid_argument("snap id"));
        }

        // let result = use_cases::get_rating(&app_ctx, snap_id).await;

        match get_votes_by_snap_id(&ctx, &snap_id, &mut conn).await {
            Ok(votes) => {
                let rating = OldRating::new(votes);
                let payload = GetRatingResponse {
                    rating: Some(rating.into()),
                };
                Ok(Response::new(payload))
            }
            Err(e) => {
                error!("Error calling get_votes_by_snap_id: {:?}", e);
                Err(Status::unknown("Internal server error"))
            }
        }
    }
}

impl From<OldRating> for Rating {
    fn from(value: OldRating) -> Rating {
        Rating {
            snap_id: value.snap_id,
            total_votes: value.total_votes,
            ratings_band: value.ratings_band as i32,
        }
    }
}
