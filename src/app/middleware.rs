use http::{header, Uri};
use hyper::service::Service;
use hyper::Body;
use std::pin::Pin;
use tonic::body::BoxBody;
use tonic::{Request, Status};
use tower::Layer;
use tracing::info;

#[derive(Debug, Clone)]
pub struct Context {
    uri: String,
}

#[tracing::instrument]
pub fn authentication(req: Request<()>) -> Result<Request<()>, Status> {
    info!("validating request authorization");

    let ctx = req.extensions().get::<Context>();

    if ctx.is_none() {
        return Err(Status::unknown("no request context"));
    }
    let ctx = ctx.unwrap();
    let uri = &ctx.uri;

    if uri.contains("ratings.feature.register.Register/Create")
        || uri.contains("grpc.reflection.v1alpha.ServerReflection/ServerReflectionInfo")
    {
        return Ok(req);
    }

    let Some(token) = req.metadata().get(header::AUTHORIZATION.as_str()) else {
        return Err(Status::unauthenticated("missing authz header"));
    };

    let token = token.to_str().unwrap_or("");

    if token.len() == crate::feature::register::TOKEN_LENGTH {
        Ok(req)
    } else {
        Err(Status::unauthenticated("invalid authz token"))
    }
}

#[derive(Debug, Clone, Default)]
pub struct ContextMiddlewareLayer;

impl<S> Layer<S> for ContextMiddlewareLayer {
    type Service = ContextMiddleware<S>;

    fn layer(&self, service: S) -> Self::Service {
        ContextMiddleware { inner: service }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ContextMiddleware<S> {
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

        Box::pin(async move {
            let uri = req.uri().clone();
            let ctx = Context {
                uri: uri.to_string(),
            };

            let mut req = req;
            req.extensions_mut().insert(ctx);

            let response = inner.call(req).await?;
            Ok(response)
        })
    }
}
