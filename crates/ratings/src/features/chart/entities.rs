//! Contains struct definitions for the charting feature for ratings.
use crate::features::{
    common::entities::{calculate_band, Rating, VoteSummary},
    pb::chart as pb,
};
use sqlx::FromRow;

/// A chart over a given [`Timeframe`] with the given [`ChartData`].
///
/// [`Timeframe`]: pb::Timeframe
pub struct Chart {
    /// The timeframe over which to display the data
    pub timeframe: pb::Timeframe,
    /// The raw chart data to display
    pub chart_data: Vec<ChartData>,
}

impl Chart {
    /// Creates a new [`Chart`] over the given [`Timeframe`]. The [`VoteSummary`] will be
    /// interpreted as [`ChartData`].
    ///
    /// If there are more than 20 elements, this will always truncate to the top 20.
    ///
    /// [`Timeframe`]: pb::Timeframe
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

/// The actual information to be charted, contains a raw calculated value
/// as well as an overall [`Rating`] with more info struct.
#[derive(Debug, Clone, FromRow)]
pub struct ChartData {
    /// The raw rating as it should be charted
    pub raw_rating: f32,
    /// A complex rating with the band, overall votes, and the snap this rating is for
    pub rating: Rating,
}

impl ChartData {
    /// Creates a single element of [`ChartData`] from a [`VoteSummary`] -- this calculates the overall
    /// rating on demand.
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

    /// Converts this [`ChartData`] into its wire version as defined in the
    /// protobuf files for transmission.
    pub fn into_protobuf_chart_data(self) -> pb::ChartData {
        pb::ChartData {
            raw_rating: self.raw_rating,
            rating: Some(self.rating.into_protobuf_rating()),
        }
    }
}
