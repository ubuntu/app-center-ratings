use std::pin::Pin;

use hyper::Body;
use hyper::service::Service;
use tonic::body::BoxBody;
use tower::Layer;

use crate::app::context::Context;

#[derive(Clone, Default)]
pub struct ContextMiddlewareLayer;

impl<S> Layer<S> for ContextMiddlewareLayer {
    type Service = ContextMiddleware<S>;
    fn layer(&self, service: S) -> Self::Service {
        ContextMiddleware {
            inner: service
        }
    }
}

#[derive(Clone)]
pub struct ContextMiddleware<S> {
    inner: S,
}

type BoxFuture<'a, T> = Pin<Box<dyn std::future::Future<Output=T> + Send + 'a>>;

impl<S> Service<hyper::Request<Body>> for ContextMiddleware<S>
    where
        S: Service<hyper::Request<Body>, Response=hyper::Response<BoxBody>> + Clone + Send + 'static,
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

    fn call(&mut self, mut req: hyper::Request<Body>) -> Self::Future {
        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);

        Box::pin(async move {
            let uri = req.uri().clone().to_string();
            let context = Context { uri, claims: None };

            let mut req = req;
            req.extensions_mut().insert(context);

            let response = inner.call(req).await?;
            Ok(response)
        })
    }
}
