use thiserror::Error;

#[derive(Error, Debug)]
pub enum RegisterError {
    #[error("invalid uid")]
    InvalidUid,
    #[error("failed to create user record")]
    FailedToCreateUserRecord,
    #[error("unknown user no auth error")]
    Unknown,
}
