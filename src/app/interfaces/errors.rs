use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppInterfaceError {
    #[error("app interface: unknown error")]
    Unknown,
}
