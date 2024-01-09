//! The middleware layers for the app context.

use std::pin::Pin;

use hyper::{service::Service, Body};
use tonic::body::BoxBody;
use tower::Layer;

use crate::app::context::{AppContext, RequestContext};

/// Passthrough [`Layer`] containing the [`AppContext`], this is mainly used to construct
/// [`ContextMiddleware`].
#[derive(Clone)]
pub struct ContextMiddlewareLayer {
    /// The wrapped context
    app_ctx: AppContext,
}

impl ContextMiddlewareLayer {
    /// Creates a new layer from the given [`AppContext`].
    pub fn new(ctx: AppContext) -> ContextMiddlewareLayer {
        ContextMiddlewareLayer { app_ctx: ctx }
    }
}

impl<S> Layer<S> for ContextMiddlewareLayer
where
    S: Service<hyper::Request<Body>, Response = hyper::Response<BoxBody>> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Service = ContextMiddleware<S>;

    fn layer(&self, service: S) -> Self::Service {
        ContextMiddleware {
            app_ctx: self.app_ctx.clone(),
            inner: service,
        }
    }
}

/// A [`Service`] which delegates responses to a request by the inner `S`,
/// which is itself a [`Service`], by calling the inner [`Future`].
///
/// [`Future`]: std::future::Future
#[derive(Clone)]
pub struct ContextMiddleware<S>
where
    S: Service<hyper::Request<Body>, Response = hyper::Response<BoxBody>> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    /// The current context of the app, as passed in from the [`ContextMiddlewareLayer`]
    app_ctx: AppContext,

    /// The inner [`Service`] containing the [`Future`].
    ///
    /// [`Future`]: std::future::Future
    inner: S,
}

/// A type definition which is simply a future that's in a pinned location in the heap.
type BoxFuture<'a, T> = Pin<Box<dyn std::future::Future<Output = T> + Send + 'a>>;

impl<S> Service<hyper::Request<Body>> for ContextMiddleware<S>
where
    S: Service<hyper::Request<Body>, Response = hyper::Response<BoxBody>> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: hyper::Request<Body>) -> Self::Future {
        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);
        let app_ctx = self.app_ctx.clone();

        Box::pin(async move {
            let req_ctx = RequestContext {
                uri: req.uri().to_string(),
                claims: None,
            };

            let mut req = req;
            req.extensions_mut().insert(app_ctx);
            req.extensions_mut().insert(req_ctx);

            let response = inner.call(req).await?;
            Ok(response)
        })
    }
}
