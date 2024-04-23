//! Contains errors for the [`Chart`] features
//!
//! [`Chart`]: crate::features::chart::entities::Chart
use thiserror::Error;

/// An error that can occur while using the chart service
#[derive(Error, Debug)]
pub enum ChartError {
    /// Could not retrieve the chart
    #[error("failed to get chart for timeframe")]
    FailedToGetChart,
    /// There was no data in the given timeframe
    #[error("could not find data for given timeframe")]
    NotFound,
    /// Another unknown error (e.g. network failure)
    #[error("unknown chart error")]
    Unknown,
}
