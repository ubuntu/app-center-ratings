//! Struct definitions for the charting feature for ratings.
use crate::{
    db::{Timeframe, VoteSummary},
    ratings::rating::{calculate_band, Rating},
};
use std::cmp::Ordering;

#[derive(Debug, Clone)]
pub struct Chart {
    pub timeframe: Timeframe,
    pub data: Vec<ChartData>,
}

impl Chart {
    pub fn new(timeframe: Timeframe, data: Vec<VoteSummary>) -> Self {
        let mut data: Vec<ChartData> = data.into_iter().map(Into::into).collect();

        data.sort_by(|a, b| {
            b.raw_rating
                .partial_cmp(&a.raw_rating)
                .unwrap_or(Ordering::Equal)
        });

        // Take only the first 20 elements from the sorted chart_data
        Chart {
            timeframe,
            data: data.into_iter().take(20).collect(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChartData {
    pub raw_rating: f32,
    pub rating: Rating,
}

impl From<VoteSummary> for ChartData {
    fn from(vote_summary: VoteSummary) -> Self {
        let (raw_rating, ratings_band) = calculate_band(&vote_summary);
        let rating = Rating {
            snap_id: vote_summary.snap_id,
            total_votes: vote_summary.total_votes as u64,
            ratings_band,
        };
        let raw_rating = raw_rating.unwrap_or(0.0) as f32;

        Self { raw_rating, rating }
    }
}
