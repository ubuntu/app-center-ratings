//! A custom Tower [Layer] for validating jwt tokens and attaching the decoded claim to incoming
//! requests.
use crate::jwt::JwtVerifier;
use http::{Request, Response};
use std::{
    error::Error,
    future::Future,
    mem::replace,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};
use tonic::Status;
use tower::{Layer, Service};

type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send + 'static>>;
type BoxError = Box<dyn Error + Send + Sync>;

/// The paths which are accessible without authentication
pub const PUBLIC_PATHS: [&str; 1] = ["ratings.features.user.User/Authenticate"];

#[derive(Clone)]
pub struct AuthLayer {
    verifier: Arc<JwtVerifier>,
}

impl AuthLayer {
    pub fn new(verifier: JwtVerifier) -> Self {
        Self {
            verifier: Arc::new(verifier),
        }
    }
}

impl<S> Layer<S> for AuthLayer {
    type Service = AuthMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        AuthMiddleware {
            inner,
            verifier: self.verifier.clone(),
        }
    }
}

#[derive(Clone)]
pub struct AuthMiddleware<S> {
    inner: S,
    verifier: Arc<JwtVerifier>,
}

// Helper for constructing the boxed errors we need to return from the Layer implementation below
macro_rules! unauthenticated {
    ($msg:expr) => {
        Box::pin(async move { Err(Box::new(Status::unauthenticated($msg)) as BoxError) })
    };
}

// The implementation here is based on the example provided by Tonic but with some type aliases and
// simplifying of a few of the generics to tailor things to our use case.
//
//   https://github.com/hyperium/tonic/blob/master/examples/src/tower/server.rs
impl<S, T, U> Service<Request<T>> for AuthMiddleware<S>
where
    S: Service<Request<T>, Response = Response<U>, Error = BoxError> + Clone + Send + 'static,
    S::Future: Send + 'static,
    T: Send + 'static,
{
    type Response = Response<U>;
    type Error = BoxError;
    type Future = BoxFuture<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<T>) -> Self::Future {
        // See: https://docs.rs/tower/latest/tower/trait.Service.html#be-careful-when-cloning-inner-services
        let clone = self.inner.clone();
        let mut inner = replace(&mut self.inner, clone);

        if !PUBLIC_PATHS.iter().any(|s| req.uri().path().ends_with(s)) {
            let header = match req.headers().get("authorization") {
                Some(h) => h.to_str().unwrap(),
                None => return unauthenticated!("missing auth header"),
            };

            let parts: Vec<&str> = header.split_whitespace().collect();
            if parts.len() != 2 {
                return unauthenticated!("malformed auth header");
            }

            match self.verifier.decode(parts[1]) {
                Ok(claims) => {
                    req.extensions_mut().insert(claims);
                }
                Err(_) => return unauthenticated!("invalid auth header"),
            }
        }

        Box::pin(async move { inner.call(req).await })
    }
}
