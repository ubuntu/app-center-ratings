//! Error implementations for the app interface.
use thiserror::Error;

/// A generic interface error, currently never constructed.
#[derive(Error, Debug)]
pub enum AppInterfaceError {
    /// Something unpredictable went wrong
    #[error("app interface: unknown error")]
    #[allow(dead_code)]
    Unknown,
}
