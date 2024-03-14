//! The middleware layers for the app context.

use std::pin::Pin;

use axum::body::Bytes;
use futures::ready;
use http_body::combinators::UnsyncBoxBody;
use hyper::{service::Service, Body};
use tower::Layer;

use crate::app::context::AppContext;

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
    S: Service<hyper::Request<Body>, Response = hyper::Response<UnsyncBoxBody<Bytes, axum::Error>>>
        + Clone
        + Send
        + 'static,
    S::Future: Send + 'static,
{
    type Service = ContextMiddleware<S>;

    fn layer(&self, service: S) -> Self::Service {
        ContextMiddleware {
            app_ctx: self.app_ctx.clone(),
            inner: service,
            ready: false,
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
    S: Service<hyper::Request<Body>, Response = hyper::Response<UnsyncBoxBody<Bytes, axum::Error>>>
        + Clone
        + Send
        + 'static,
    S::Future: Send + 'static,
{
    /// The current context of the app, as passed in from the [`ContextMiddlewareLayer`]
    app_ctx: AppContext,

    /// Is the inner future ready to be used
    ready: bool,

    /// The inner [`Service`] containing the [`Future`].
    ///
    /// [`Future`]: std::future::Future
    inner: S,
}

/// A type definition which is simply a future that's in a pinned location in the heap.
type BoxFuture<'a, T> = Pin<Box<dyn std::future::Future<Output = T> + Send + 'a>>;

impl<S> Service<hyper::Request<Body>> for ContextMiddleware<S>
where
    S: Service<hyper::Request<Body>, Response = hyper::Response<UnsyncBoxBody<Bytes, axum::Error>>>
        + Clone
        + Send
        + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        loop {
            if self.ready {
                return std::task::Poll::Ready(Ok(()));
            } else {
                ready!(self.inner.poll_ready(cx))?;
                self.ready = true;
            }
        }
    }

    fn call(&mut self, mut req: hyper::Request<Body>) -> Self::Future {
        assert!(self.ready);
        self.ready = false;

        req.extensions_mut().insert(self.app_ctx.clone());

        let future = self.inner.call(req);

        Box::pin(async move {
            let res = future.await?;
            Ok(res)
        })
    }
}
