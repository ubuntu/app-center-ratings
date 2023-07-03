use thiserror::Error;

#[derive(Error, Debug)]
pub enum UserError {
    #[error("unknown user error")]
    Unknown,
}
