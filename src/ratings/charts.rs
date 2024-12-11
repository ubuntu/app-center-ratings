//! Struct definitions for the charting feature for ratings.
use futures::future::try_join_all;

use crate::{
    db::{Timeframe, VoteSummary},
    ratings::{
        get_snap_name,
        rating::{calculate_band, Rating},
        Error,
    },
    Context,
};
use std::cmp::Ordering;

#[derive(Debug, Clone)]
pub struct Chart {
    pub timeframe: Timeframe,
    pub data: Vec<ChartData>,
}

impl Chart {
    pub async fn new(
        timeframe: Timeframe,
        data: Vec<VoteSummary>,
        ctx: &Context,
    ) -> Result<Self, Error> {
        let mut data: Vec<ChartData> = try_join_all(data.into_iter().map(|vote_summary| async {
            let snap_name = get_snap_name(
                &vote_summary.snap_id,
                &ctx.config.snapcraft_io_uri,
                &ctx.http_client,
            )
            .await?;

            Result::<ChartData, Error>::Ok(ChartData::new(vote_summary, &snap_name))
        }))
        .await?;

        data.sort_by(|a, b| {
            b.raw_rating
                .partial_cmp(&a.raw_rating)
                .unwrap_or(Ordering::Equal)
        });

        // Take only the first 20 elements from the sorted chart_data
        Ok(Self {
            timeframe,
            data: data.into_iter().take(20).collect(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct ChartData {
    pub raw_rating: f32,
    pub rating: Rating,
}

impl ChartData {
    pub fn new(vote_summary: VoteSummary, snap_name: &str) -> Self {
        let (raw_rating, ratings_band) = calculate_band(&vote_summary);
        let rating = Rating {
            snap_id: vote_summary.snap_id,
            total_votes: vote_summary.total_votes as u64,
            ratings_band,
            snap_name: snap_name.into(),
        };
        let raw_rating = raw_rating.unwrap_or(0.0) as f32;

        Self { raw_rating, rating }
    }
}
