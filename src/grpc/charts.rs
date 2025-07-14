use crate::{
    conn,
    db::{Category, Timeframe, VoteSummary},
    grpc::app::populate_chart_data_with_names,
    proto::{
        chart::{
            chart_server::{self, ChartServer},
            GetChartRequest, GetChartResponse,
        },
        common::{ChartData as PbChartData, Rating as PbRating, RatingsBand as PbRatingsBand},
    },
    ratings::{Chart, ChartData, Rating, RatingsBand},
    Context,
};
use cached::proc_macro::cached;
use std::sync::Arc;
use tonic::{Request, Response, Status};
use tracing::error;

#[derive(Clone)]
pub struct ChartService {
    ctx: Arc<Context>,
}

impl ChartService {
    pub fn new_server(ctx: Arc<Context>) -> ChartServer<ChartService> {
        ChartServer::new(Self { ctx })
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

        let chart = get_chart_cached(category, timeframe).await;

        match chart {
            Ok(chart) if chart.data.is_empty() => {
                Err(Status::not_found("Cannot find data for given timeframe."))
            }

            Ok(chart) => {
                let ordered_chart_data =
                    populate_chart_data_with_names(&self.ctx, chart.data).await?;

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

#[cfg_attr(not(feature = "skip_cache"), cached(
    time = 86400, // 24 hours
    sync_writes = true,
    key = "String",
    convert = r##"{format!("{:?}{:?}", category, timeframe)}"##,
    result = true,
))]
async fn get_chart_cached(
    category: Option<Category>,
    timeframe: Timeframe,
) -> Result<Chart, crate::db::Error> {
    let summaries = VoteSummary::get_for_timeframe(timeframe, category, conn!()).await?;

    Ok(Chart::new(timeframe, summaries))
}

impl PbChartData {
    pub fn from_chart_data_and_snap_name(chart_data: ChartData, snap_name: String) -> Self {
        Self {
            raw_rating: chart_data.raw_rating,
            rating: Some(PbRating::from_rating_and_snap_name(
                chart_data.rating,
                snap_name,
            )),
        }
    }
}

impl PbRating {
    fn from_rating_and_snap_name(rating: Rating, snap_name: String) -> Self {
        Self {
            snap_id: rating.snap_id,
            total_votes: rating.total_votes,
            ratings_band: rating.ratings_band as i32,
            snap_name,
        }
    }
}

impl From<PbRating> for Rating {
    fn from(r: PbRating) -> Self {
        Self {
            snap_id: r.snap_id,
            total_votes: r.total_votes,
            ratings_band: RatingsBand::from_repr(r.ratings_band).unwrap(),
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
