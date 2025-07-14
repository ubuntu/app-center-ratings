use crate::{
    conn,
    db::{Timeframe, VoteSummary},
    proto::{
        app::{
            app_server::{App, AppServer},
            GetBulkRatingsRequest, GetBulkRatingsResponse, GetRatingRequest, GetRatingResponse,
        },
        common::{ChartData as PbChartData, Rating as PbRating},
    },
    ratings::{get_snap_name, Chart, ChartData, Error as RatingsError, Rating},
    Context,
};
use futures::future::try_join_all;
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
        let GetBulkRatingsRequest { snap_ids } = request.into_inner();

        if snap_ids.is_empty() {
            return Err(Status::invalid_argument("snap_ids cannot be empty"));
        }

        const MAX_IDS: usize = 250;
        if snap_ids.len() > MAX_IDS {
            return Err(Status::invalid_argument(format!(
                "Too many snap_ids requested. The maximum is {}",
                MAX_IDS
            )));
        }

        const TIMEFRAME: Timeframe = Timeframe::Month;

        let vote_summaries = VoteSummary::get_by_snap_ids(&snap_ids, TIMEFRAME, conn!())
            .await
            .map_err(|e| {
                error!("Error calling get_by_snap_ids: {:?}", e);
                Status::unknown("Internal server error")
            })?;

        let chart = Chart::new(
            TIMEFRAME,
            vote_summaries.into_iter().map(Into::into).collect(),
        );

        let ratings = populate_chart_data_with_names(&self.ctx, chart.data).await?;

        Ok(Response::new(GetBulkRatingsResponse { ratings }))
    }
}

pub async fn populate_chart_data_with_names(
    ctx: &Arc<Context>,
    chart_data_vec: Vec<ChartData>,
) -> Result<Vec<PbChartData>, Status> {
    try_join_all(chart_data_vec.into_iter().map(|chart_data| async {
        let snap_name = get_snap_name(
            &chart_data.rating.snap_id,
            &ctx.config.snapcraft_io_uri,
            &ctx.http_client,
        )
        .await
        .map_err(|e| {
            let mut err = &e as &dyn Error;
            let mut error_chain = format!("{err}");
            while let Some(src) = err.source() {
                error_chain.push_str(&format!("\nCaused by: {src}"));
                err = src;
            }
            error!(error=%error_chain, "unable to fetch snap name");
            Status::unknown("Internal server error")
        })?;

        Ok(PbChartData::from_chart_data_and_snap_name(
            chart_data, snap_name,
        ))
    }))
    .await
}
