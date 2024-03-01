//! The interface for serving on REST endpoints

use std::{convert::Infallible, pin::Pin};

use axum::{body::Bytes, response::IntoResponse, Router};
use http_body::combinators::UnsyncBoxBody;
use hyper::StatusCode;
use tower::Service;

use crate::features::admin::log_level::service::LogLevelService;

/// The base path appended to all our internal endpoints
const BASE_ROUTE: &str = "/v1/";

/// Dispatches to our web endpoints
#[derive(Clone, Debug)]
pub struct RestService {
    /// The axum router we use for dispatching to endpoints
    router: Router,
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

    fn call(&mut self, req: hyper::Request<hyper::Body>) -> Self::Future {
        let future = self.router.call(req);
        Box::pin(future)
    }
}

/// Handles any missing paths
async fn handler_404() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, "no such API endpoint")
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

    /// Builds the REST service, applying all configured paths and
    /// forcing the others to 404.
    pub fn build(self) -> RestService {
        RestService {
            router: self.router.fallback(handler_404),
        }
    }
}

impl Default for RestServiceBuilder {
    fn default() -> Self {
        Self::new().with_log_level()
    }
}
