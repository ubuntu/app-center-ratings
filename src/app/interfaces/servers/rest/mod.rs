//! The interface for serving on REST endpoints

use std::{convert::Infallible, pin::Pin};

use axum::{body::Bytes, response::IntoResponse, Router};
use http_body::combinators::UnsyncBoxBody;
use hyper::StatusCode;
use thiserror::Error;
use tower::Service;

use crate::{
    app::interfaces::authentication::{
        admin::{AdminAuthError, AdminAuthVerifier},
        Authenticator, AuthenticatorBuilder,
    },
    features::admin::{
        api_version::service::ApiVersionService, log_level::service::LogLevelService,
    },
};

/// The base path appended to all our internal endpoints
const BASE_ROUTE: &str = "/v1/";

/// Dispatches to our web endpoints
#[derive(Clone)]
pub struct RestService {
    /// The axum router we use for dispatching to endpoints
    router: Router,
    /// Makes sure our admin endpoints aren't public without some kind of
    /// username and password to access them.
    authenticator: Authenticator<AdminAuthVerifier, &'static str>,
}

/// A type definition which is simply a future that's in a pinned location in the heap.
type BoxFuture<'a, T> = Pin<Box<dyn std::future::Future<Output = T> + Send + 'a>>;

impl Service<hyper::Request<hyper::Body>> for RestService {
    type Response = hyper::Response<UnsyncBoxBody<Bytes, axum::Error>>;

    type Error = Infallible;

    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.router
            .poll_ready(cx)
            .map_err(|_| unreachable!("error is infallible"))
    }

    fn call(&mut self, mut req: hyper::Request<hyper::Body>) -> Self::Future {
        let auth_result = self.authenticator.authenticate(&mut req);

        if let Err(err) = auth_result {
            return Box::pin(async move { Ok(err.into_response()) });
        };

        let future = self.router.call(req);
        Box::pin(async move {
            let resp = future
                .await
                .map_err(|_| unreachable!("error is infallible"))
                .unwrap();

            Ok(resp.into_response())
        })
    }
}

/// Handles any missing paths
async fn handler_404() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, "no such API endpoint")
}

/// Errors that can occur while constructing our REST service
#[derive(Error, Debug)]
#[allow(clippy::missing_docs_in_private_items, missing_docs)]
pub enum RestServerBuildError {
    #[error("grpc builder: error creating admin authentication: {0}")]
    JwtDecodeError(#[from] AdminAuthError),
}

/// Builds the REST service
pub struct RestServiceBuilder {
    /// The underlying axum router we're building up
    router: Router,
}

impl RestServiceBuilder {
    /// Creates a new builder with an empty path,
    /// you probably actually want [`RestServiceBuilder::default`],
    /// since that seeds the default API endpoint paths.
    pub fn new() -> Self {
        Self {
            router: Router::default(),
        }
    }

    /// Adds the log service
    pub fn with_log_level(self) -> Self {
        Self {
            router: self
                .router
                .nest(BASE_ROUTE, LogLevelService.register_axum_route()),
        }
    }

    /// Adds the ability to get the API version from the REST endpoint
    pub fn with_api_version(self) -> Self {
        Self {
            router: self
                .router
                .nest(BASE_ROUTE, ApiVersionService.register_axum_route()),
        }
    }

    /// Builds the REST service, applying all configured paths and
    /// forcing the others to 404.
    pub fn build(self) -> Result<RestService, RestServerBuildError> {
        Ok(RestService {
            router: self.router.fallback(handler_404),
            // None of our paths are public right now, so
            authenticator: AuthenticatorBuilder::new(AdminAuthVerifier::from_env()?).build(),
        })
    }
}

impl Default for RestServiceBuilder {
    fn default() -> Self {
        Self::new().with_log_level().with_api_version()
    }
}
