//! API endpoint definitions for different entry methods

pub mod grpc;
pub mod rest;

use std::convert::Infallible;
use std::pin::Pin;

use axum::body::Bytes;
use axum::response::IntoResponse;
use futures::ready;
#[allow(unused_imports)]
pub use grpc::{GrpcService, GrpcServiceBuilder};
use http_body::combinators::UnsyncBoxBody;
use hyper::{header::CONTENT_TYPE, Request};
pub use rest::{RestService, RestServiceBuilder};
use thiserror::Error;
use tower::Service;

use self::grpc::GrpcError;

/// Any error that can occur internally to our service
#[derive(Debug, Error)]
pub enum AppCenterRatingsError {
    /// An error from the GRPC endpoints
    #[error("an error from the GRPC service occurred: {0}")]
    GrpcError(#[from] GrpcError),
    /// Technically, an error from the Rest endpoints, but they're infallible
    #[error("cannot happen")]
    RestError(#[from] Infallible),
}

/// The general service for our app, containing all our endpoints
#[derive(Clone)]
#[allow(clippy::missing_docs_in_private_items)]
pub struct AppCenterRatingsService {
    grpc_service: GrpcService,
    grpc_ready: bool,
    rest_service: RestService,
    rest_ready: bool,
}

impl AppCenterRatingsService {
    /// Constructs the service with all the default service endpoints for REST and GRPC
    pub fn with_default_routes() -> AppCenterRatingsService {
        Self {
            grpc_service: GrpcServiceBuilder::from_env()
                .expect("could not create GRPC service from environment")
                .with_default_routes()
                .build(),
            grpc_ready: false,
            rest_service: RestServiceBuilder::default().build(),
            rest_ready: false,
        }
    }
}

/// A type definition which is simply a future that's in a pinned location in the heap.
type BoxFuture<'a, T> = Pin<Box<dyn std::future::Future<Output = T> + Send + 'a>>;

impl Service<hyper::Request<hyper::Body>> for AppCenterRatingsService {
    type Response = hyper::Response<UnsyncBoxBody<Bytes, axum::Error>>;

    type Error = AppCenterRatingsError;

    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        loop {
            match (self.grpc_ready, self.rest_ready) {
                (true, true) => return std::task::Poll::Ready(Ok(())),
                (false, _) => {
                    ready!(self.grpc_service.poll_ready(cx))?;
                    self.grpc_ready = true
                }
                (_, false) => {
                    ready!(self.rest_service.poll_ready(cx)).unwrap();
                    self.rest_ready = true
                }
            }
        }
    }

    fn call(&mut self, req: hyper::Request<hyper::Body>) -> Self::Future {
        assert!(
            self.grpc_ready,
            "grpc service not ready. Did you forget to call `poll_ready`?"
        );
        assert!(
            self.rest_ready,
            "rest service not ready. Did you forget to call `poll_ready`?"
        );

        // if we get a grpc request call the grpc service, otherwise call the rest service
        // when calling a service it becomes not-ready so we have drive readiness again
        if is_grpc_request(&req) {
            self.grpc_ready = false;
            let future = self.grpc_service.call(req);
            Box::pin(async move {
                let res = future.await?;
                Ok(res.into_response())
            })
        } else {
            self.rest_ready = false;
            let future = self.rest_service.call(req);
            Box::pin(async move {
                let res = future.await?;
                Ok(res.into_response())
            })
        }
    }
}

/// Checks to see if this request has a GRPC header (if not we assume REST)
fn is_grpc_request<B>(req: &Request<B>) -> bool {
    req.headers()
        .get(CONTENT_TYPE)
        .map(|content_type| content_type.as_bytes())
        .filter(|content_type| content_type.starts_with(b"application/grpc"))
        .is_some()
}
