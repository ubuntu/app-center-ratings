use sqlx::FromRow;

use crate::features::common::entities::{calculate_band, Rating, VoteSummary};
use crate::features::pb::chart as pb;

pub struct Chart {
    pub timeframe: pb::Timeframe,
    pub chart_data: Vec<ChartData>,
}

impl Chart {
    pub fn new(timeframe: pb::Timeframe, data: Vec<VoteSummary>) -> Self {
        let mut chart_data: Vec<ChartData> =
            data.into_iter().map(ChartData::from_vote_summary).collect();

        chart_data.sort_by(|a, b| {
            b.raw_rating
                .partial_cmp(&a.raw_rating)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Take only the first 20 elements from the sorted chart_data
        let top_20: Vec<ChartData> = chart_data.into_iter().take(20).collect();

        Chart {
            timeframe,
            chart_data: top_20,
        }
    }
}

#[derive(Debug, Clone, FromRow)]
pub struct ChartData {
    pub raw_rating: f32,
    pub rating: Rating,
}

impl ChartData {
    pub fn from_vote_summary(vote_summary: VoteSummary) -> Self {
        let (raw_rating, ratings_band) = calculate_band(&vote_summary);
        let rating = Rating {
            snap_id: vote_summary.snap_id,
            total_votes: vote_summary.total_votes as u64,
            ratings_band,
        };
        let raw_rating = raw_rating.unwrap_or(0.0) as f32;
        Self { raw_rating, rating }
    }

    pub fn into_dto(self) -> pb::ChartData {
        pb::ChartData {
            raw_rating: self.raw_rating,
            rating: Some(self.rating.into_dto()),
        }
    }
}
