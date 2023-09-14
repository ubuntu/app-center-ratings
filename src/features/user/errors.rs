use thiserror::Error;

#[derive(Error, Debug)]
pub enum UserError {
    #[error("failed to create user record")]
    FailedToCreateUserRecord,
    #[error("failed to delete user by instance id")]
    FailedToDeleteUserRecord,
    #[error("failed to get user vote")]
    FailedToGetUserVote,
    #[error("failed to cast vote")]
    FailedToCastVote,
    #[error("unknown user error")]
    Unknown,
}
