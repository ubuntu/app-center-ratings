//! Contains definitions for service routers and related components.

use std::pin::Pin;

use hyper::Body;
use thiserror::Error;
use tonic::{
    transport::server::{Routes, RoutesBuilder},
    Status,
};
use tower::Service;

use crate::features::{chart::ChartService, rating::RatingService, user::UserService};

mod authentication;

use authentication::GrpcAuthenticator;

/// An error deriving from the GRPC Endpoints
#[derive(Error, Debug)]
pub enum GrpcError {
    /// The [`tonic`] API erases individual responses from our underlying routes,
    /// so this collects those.
    #[error("an error occurred in an underlying service: {0}")]
    RoutesError(#[from] Box<dyn std::error::Error + Send + Sync + 'static>),
    /// Errors hailing from our authentication interceptor
    #[error("an error occurred during authentication: {0}")]
    AuthError(#[from] tonic::Status),
}

impl From<GrpcError> for Status {
    fn from(value: GrpcError) -> Self {
        match value {
            GrpcError::AuthError(status) => status,
            GrpcError::RoutesError(err) => Status::internal(format!("{err}")),
        }
    }
}

/// The file descriptors defining the [`tonic`] GRPC service
const FILE_DESCRIPTOR_SET: &[u8] = tonic::include_file_descriptor_set!("ratings_descriptor");

/// The GRPC Service endpoint for the program, you probably want to build this with
/// [`GrpcServiceBuilder`] instead of using this directly.
#[derive(Default, Debug, Clone)]
pub struct GrpcService {
    /// The router that automatically sends requests to the proper underlying service
    routes: Routes,
    /// The authentication routine we use for validating input
    authenticator: GrpcAuthenticator,
}

/// A type definition which is simply a future that's in a pinned location in the heap.
type BoxFuture<'a, T> = Pin<Box<dyn std::future::Future<Output = T> + Send + 'a>>;

impl Service<hyper::Request<Body>> for GrpcService {
    type Response = hyper::Response<tonic::body::BoxBody>;

    type Error = GrpcError;

    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.routes.poll_ready(cx).map_err(|e| e.into())
    }

    fn call(&mut self, mut req: hyper::Request<Body>) -> Self::Future {
        let auth_result = self.authenticator.authenticate(&mut req);

        if let Err(err) = auth_result {
            return Box::pin(async move { Err(err)? });
        };

        let future = self.routes.call(req);
        Box::pin(async move { Ok(future.await?) })
    }
}

/// A builder for the ratings GRPC backend
pub struct GrpcServiceBuilder {
    /// The builder for the service's route dispatcher
    builder: RoutesBuilder,
    /// The authenticator we want to use
    authenticator: GrpcAuthenticator,
}

impl GrpcServiceBuilder {
    /// Creates a new builder for our GrpcService
    pub fn new() -> GrpcServiceBuilder {
        GrpcServiceBuilder {
            builder: RoutesBuilder::default(),
            authenticator: GrpcAuthenticator,
        }
    }

    /// Adds the [`ChartService`] to the [`GrpcService`]
    pub fn with_charts(mut self) -> Self {
        self.builder.add_service(ChartService.to_server());
        self
    }

    /// Adds the [`RatingService`] to the [`GrpcService`]
    pub fn with_ratings(mut self) -> Self {
        self.builder.add_service(RatingService.to_server());
        self
    }

    /// Adds the [`UserService`] to the [`GrpcService`]
    pub fn with_user(mut self) -> Self {
        self.builder.add_service(UserService.to_server());
        self
    }

    /// Adds the tonic [`ServerReflectionServer`] to the [`GrpcService`]
    ///
    /// [`ServerReflectionServer`]: tonic_reflection::server::ServerReflectionServer
    pub fn with_reflection(mut self) -> Self {
        self.builder.add_service(
            tonic_reflection::server::Builder::configure()
                .register_encoded_file_descriptor_set(FILE_DESCRIPTOR_SET)
                .build()
                .unwrap(),
        );
        self
    }

    /// Adds the given [`GrpcAuthenticator`] to the [`GrpcService`],
    /// this is largely pointless because at the moment it has no configuration,
    /// but is here for posterity.  
    #[allow(dead_code)]
    pub fn with_authenticator(mut self, authenticator: GrpcAuthenticator) -> Self {
        self.authenticator = authenticator;
        self
    }

    /// Converts the builder into its underlying routes with the given client
    pub fn routes(self) -> Routes {
        self.builder.routes()
    }

    /// Builds this into the GrpcService
    pub fn build(self) -> GrpcService {
        GrpcService {
            authenticator: self.authenticator,
            routes: self.routes(),
        }
    }
}

impl Default for GrpcServiceBuilder {
    fn default() -> Self {
        Self::new()
            .with_charts()
            .with_ratings()
            .with_user()
            .with_reflection()
    }
}
