//! Contains API endpoints for getting build target information for the currently running service

use serde::{Deserialize, Serialize};

pub mod interface;
pub mod service;

#[derive(Copy, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
/// The response returning the API Version information.
pub struct ApiVersion<'a> {
    /// The current API Version
    version: &'a str,
    /// The current commit sha
    commit: &'a str,
    /// The current branch
    branch: &'a str,
}

impl<'a> ApiVersion<'a> {
    /// Retrieves the baked-in build info for the current branch state
    pub const fn build_info() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION"),
            commit: env!("GIT_HASH"),
            branch: env!("GIT_BRANCH"),
        }
    }
}

impl<'a> Default for ApiVersion<'a> {
    fn default() -> Self {
        Self::build_info()
    }
}
