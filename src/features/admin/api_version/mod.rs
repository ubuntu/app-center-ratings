//! Contains API endpoints for getting build target information for the currently running service

use std::borrow::Cow;

use serde::{Deserialize, Serialize};

pub mod interface;
pub mod service;

#[derive(Clone, Hash, PartialEq, Eq, Serialize, Deserialize, Debug)]
/// The response returning the API Version information.
pub struct ApiVersion<'a> {
    /// The current API Version
    pub version: Cow<'a, str>,
    /// The current commit sha
    pub commit: Cow<'a, str>,
    /// The current branch
    pub branch: Cow<'a, str>,
}

impl ApiVersion<'static> {
    /// Retrieves the baked-in build info for the current branch state
    pub const fn build_info() -> ApiVersion<'static> {
        Self {
            version: Cow::Borrowed(env!("CARGO_PKG_VERSION")),
            commit: Cow::Borrowed("GIT_HASH"),
            branch: Cow::Borrowed("GIT_BRANCH"),
        }
    }
}

impl Default for ApiVersion<'static> {
    fn default() -> Self {
        Self::build_info()
    }
}
