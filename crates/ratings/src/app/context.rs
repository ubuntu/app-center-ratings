//! Contains definitions for maintaining the current [`AppContext`].

use std::sync::Arc;

use crate::utils::{jwt::JwtEncoder, Config, Infrastructure};

/// An atomically reference counted app state.
#[derive(Clone)]
pub struct AppContext(Arc<AppContextInner>);

#[allow(dead_code)]
impl AppContext {
    /// Builds a new [`AppContext`] from a user [`Config`] and [`Infrastructure`].
    ///
    /// The [`Config`] will be cloned.
    pub fn new(config: &Config, infra: Infrastructure) -> Self {
        let jwt_encoder = JwtEncoder::from_config(config).unwrap();
        let jwt_encoder = Arc::new(jwt_encoder);
        let inner = AppContextInner {
            infra,
            config: config.clone(),
            http_client: reqwest::Client::new(),
            jwt_encoder,
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

    /// A reference to the JWT Encoder
    pub fn jwt_encoder(&self) -> &JwtEncoder{
        &self.0.jwt_encoder
    }
}

/// Contains the overall state and configuration of the app, only meant to be used
/// in terms of the [`AppContext`].
struct AppContextInner {
    /// Contains JWT and postgres infrastructure for the app.
    infra: Infrastructure,
    /// App configuration settings.
    config: Config,
    /// An HTTP client for pulling data from snapcraft.io
    http_client: reqwest::Client,
    /// A JWT encoder for authentication
    jwt_encoder: Arc<JwtEncoder>
}
