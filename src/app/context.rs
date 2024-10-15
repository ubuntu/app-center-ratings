//! Contains definitions for maintaining the current [`AppContext`].

use std::sync::Arc;

use crate::utils::{Config, Infrastructure};

/// An atomically reference counted app state.
#[derive(Debug, Clone)]
pub struct AppContext(Arc<AppContextInner>);

#[allow(dead_code)]
impl AppContext {
    /// Builds a new [`AppContext`] from a user [`Config`] and [`Infrastructure`].
    ///
    /// The [`Config`] will be cloned.
    pub fn new(config: &Config, infra: Infrastructure) -> Self {
        let inner = AppContextInner {
            infra,
            config: config.clone(),
            http_client: reqwest::Client::new(),
        };

        Self(Arc::new(inner))
    }

    /// A reference to the held [`Infrastructure`].
    pub fn infrastructure(&self) -> &Infrastructure {
        &self.0.infra
    }

    /// A reference to the held [`Config`].
    pub fn config(&self) -> &Config {
        &self.0.config
    }

    /// A reference to the shared HTTP client
    pub fn http_client(&self) -> &reqwest::Client {
        &self.0.http_client
    }
}

/// Contains the overall state and configuration of the app, only meant to be used
/// in terms of the [`AppContext`].
#[derive(Debug)]
struct AppContextInner {
    /// Contains JWT and postgres infrastructure for the app.
    infra: Infrastructure,
    /// App configuration settings.
    config: Config,
    /// An HTTP client for pulling data from snapcraft.io
    http_client: reqwest::Client,
}
