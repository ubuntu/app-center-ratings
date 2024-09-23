//! Errors related to user voting
use thiserror::Error;

/// Errors that can occur when a user votes.
#[derive(Error, Debug)]
pub enum UserError {
    /// A record could not be created for the user
    #[error("failed to create user record")]
    FailedToCreateUserRecord,

    /// We were unable to delete a user with the given instance ID
    #[error("failed to delete user by instance id")]
    FailedToDeleteUserRecord,

    /// We could not get a vote by a given user
    #[error("failed to get user vote")]
    FailedToGetUserVote,

    /// The user was unable to cast a vote
    #[error("failed to cast vote")]
    FailedToCastVote,

    /// An error that occurred in category updating
    #[error("an error occurred with the DB when getting categories: {0}")]
    CategoryDBError(#[from] sqlx::Error),

    /// An error that occurred while trying to pull data from snapcraft.io
    #[error(transparent)]
    SnapcraftIo(#[from] reqwest::Error),

    /// An error that occurred while trying to convert JSON data
    #[error(transparent)]
    Json(#[from] serde_json::Error),

    /// Anything else that can go wrong
    #[error("unknown user error")]
    Unknown,
}
