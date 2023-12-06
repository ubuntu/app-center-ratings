use thiserror::Error;

#[derive(Error, Debug)]
pub enum ChartError {
    #[error("failed to get chart for timeframe")]
    FailedToGetChart,
    #[error("unknown chart error")]
    Unknown,
    #[error("could not find data for given timeframe")]
    NotFound,
}
