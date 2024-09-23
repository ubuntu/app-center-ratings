//! The definitions for the interface between us and the outside world

use log::Level;
use serde::{Deserialize, Serialize};

/// The request for setting the log level
#[derive(Copy, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SetLogLevelRequest {
    /// The current log level, [`tracing`] doesn't implement [`serde`] traits so
    // we convert between the two internally.
    pub level: Level,
}

/// The response for setting the log level, essentially nothing but an Ack
#[derive(Copy, Clone, Serialize)]
pub struct SetLogLevelResponse;

/// Returns the log level to the caller
#[derive(Copy, Clone, Serialize, Deserialize)]
pub struct GetLogLevelResponse {
    /// The current log level, [`tracing`] doesn't implement [`serde`] traits so
    // we convert between the two internally.
    pub level: Level,
}
