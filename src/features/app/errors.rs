use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("failed to get rating for snap")]
    FailedToGetRating,
    #[error("unknown user error")]
    Unknown,
}
