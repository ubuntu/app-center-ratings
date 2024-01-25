//! Error definitions for the ratings feature.

use thiserror::Error;

/// Various app errors
#[derive(Error, Debug)]
pub enum AppError {
    /// Could not get a rating for the snap
    #[error("failed to get rating for snap")]
    FailedToGetRating,
    /// Unknown user error
    #[error("unknown user error")]
    Unknown,
}
