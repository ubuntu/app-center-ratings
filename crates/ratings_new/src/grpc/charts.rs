use crate::{
    conn,
    db::{Category, Timeframe, VoteSummary},
    proto::{
        chart::{
            chart_server::{self, ChartServer},
            ChartData as PbChartData, GetChartRequest, GetChartResponse,
        },
        common::{Rating as PbRating, RatingsBand as PbRatingsBand},
    },
    ratings::{Chart, ChartData, Rating, RatingsBand},
};
use tonic::{Request, Response, Status};
use tracing::error;

#[derive(Copy, Clone, Debug)]
pub struct ChartService;

impl ChartService {
    /// The paths which are accessible without authentication, if any
    pub const PUBLIC_PATHS: [&'static str; 0] = [];

    /// Converts this service into its corresponding server
    pub fn to_server(self) -> ChartServer<ChartService> {
        ChartServer::new(self)
    }
}

#[tonic::async_trait]
impl chart_server::Chart for ChartService {
    async fn get_chart(
        &self,
        request: Request<GetChartRequest>,
    ) -> Result<Response<GetChartResponse>, Status> {
        let GetChartRequest {
            timeframe,
            category,
        } = request.into_inner();

        let category = match category {
            Some(c) => Some(
                Category::from_repr(c).ok_or(Status::invalid_argument("invalid category value"))?,
            ),
            None => None,
        };

        let timeframe = Timeframe::from_repr(timeframe).unwrap_or(Timeframe::Unspecified);
        let result = VoteSummary::get_for_timeframe(timeframe, category, conn!()).await;

        match result {
            Ok(summaries) if summaries.is_empty() => {
                Err(Status::not_found("Cannot find data for given timeframe."))
            }

            Ok(summaries) => {
                let chart = Chart::new(timeframe, summaries);
                let ordered_chart_data = chart.data.into_iter().map(|cd| cd.into()).collect();

                let payload = GetChartResponse {
                    timeframe: timeframe as i32,
                    category: category.map(|c| c as i32),
                    ordered_chart_data,
                };

                Ok(Response::new(payload))
            }

            Err(e) => {
                error!("unable to fetch vote summary: {e}");
                Err(Status::unknown("Internal server error"))
            }
        }
    }
}

impl From<ChartData> for PbChartData {
    fn from(value: ChartData) -> Self {
        Self {
            raw_rating: value.raw_rating,
            rating: Some(value.rating.into()),
        }
    }
}

impl From<Rating> for PbRating {
    fn from(r: Rating) -> Self {
        Self {
            snap_id: r.snap_id,
            total_votes: r.total_votes,
            ratings_band: r.ratings_band as i32,
        }
    }
}

impl From<RatingsBand> for PbRatingsBand {
    fn from(rb: RatingsBand) -> Self {
        match rb {
            RatingsBand::VeryGood => Self::VeryGood,
            RatingsBand::Good => Self::Good,
            RatingsBand::Neutral => Self::Neutral,
            RatingsBand::Poor => Self::Poor,
            RatingsBand::VeryPoor => Self::VeryPoor,
            RatingsBand::InsufficientVotes => Self::InsufficientVotes,
        }
    }
}
