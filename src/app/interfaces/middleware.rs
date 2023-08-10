use std::pin::Pin;

use hyper::service::Service;
use hyper::Body;
use tonic::body::BoxBody;
use tower::Layer;

use crate::app::context::{AppContext, RequestContext};

#[derive(Clone)]
pub struct ContextMiddlewareLayer {
    app_ctx: AppContext,
}

impl ContextMiddlewareLayer {
    pub fn new(ctx: AppContext) -> ContextMiddlewareLayer {
        ContextMiddlewareLayer { app_ctx: ctx }
    }
}

impl<S> Layer<S> for ContextMiddlewareLayer {
    type Service = ContextMiddleware<S>;
    fn layer(&self, service: S) -> Self::Service {
        ContextMiddleware {
            app_ctx: self.app_ctx.clone(),
            inner: service,
        }
    }
}

#[derive(Clone)]
pub struct ContextMiddleware<S> {
    app_ctx: AppContext,
    inner: S,
}

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
