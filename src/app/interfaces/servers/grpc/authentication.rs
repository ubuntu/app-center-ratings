//! Contains the utilities for authorizing user requests

use http::header;
use hyper::Request;
use tonic::Status;
use tracing::error;

use crate::app::context::AppContext;

/// Authentication for a GRPC Server, it validates any passed in Grpc JWT client token
#[derive(Default, Debug, Copy, Clone)]
pub struct GrpcAuthenticator;

impl GrpcAuthenticator {
    /// Our public API paths, in the future, could refactor into a `Vec`
    /// if needed, but this should remain relatively constant.
    const PUBLIC_PATHS: [&'static str; 3] = [
        "ratings.features.user.User/Register",
        "ratings.features.user.User/Authenticate",
        "grpc.reflection.v1alpha.ServerReflection/ServerReflectionInfo",
    ];

    /// Authenticates the given HTTP Request header, on success, this modifies the underlying
    /// request with its JWT [`Claims`]
    ///
    /// [`Claims`]: crate::utils::jwt::Claims
    pub fn authenticate(&self, req: &mut Request<hyper::Body>) -> Result<(), Status> {
        let app_ctx = req.extensions().get::<AppContext>().unwrap().clone();

        let uri = req.uri().to_string();

        if Self::PUBLIC_PATHS.iter().any(|&s| uri.contains(s)) {
            return Ok(());
        }

        let Some(header) = req.headers().get(header::AUTHORIZATION.as_str()) else {
            let error = Err(Status::unauthenticated("missing authz header"));
            error!("{error:?}");
            return error;
        };

        let raw: Vec<&str> = header.to_str().unwrap_or("").split_whitespace().collect();

        if raw.len() != 2 {
            let error = Err(Status::unauthenticated("invalid authz token"));
            error!("{error:?}");
            return error;
        }

        let token = raw[1];
        let infra = app_ctx.infrastructure();
        match infra.jwt.decode(token) {
            Ok(claim) => {
                req.extensions_mut().insert(claim);
                Ok(())
            }
            Err(error) => {
                error!("{error:?}");
                Err(Status::unauthenticated("Failed to decode token."))
            }
        }
    }
}
