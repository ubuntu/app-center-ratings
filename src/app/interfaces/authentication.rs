use http::header;
use tonic::{Request, Status};
use tracing::error;

use crate::app::context::AppContext;
use crate::app::RequestContext;

pub fn authentication(req: Request<()>) -> Result<Request<()>, Status> {
    let app_ctx = req.extensions().get::<AppContext>().unwrap().clone();

    let req_ctx = req.extensions().get::<RequestContext>().unwrap().clone();
    let uri = &req_ctx.uri;

    let public_paths = [
        "ratings.features.user.User/Register",
        "ratings.features.user.User/Authenticate",
        "grpc.reflection.v1alpha.ServerReflection/ServerReflectionInfo",
    ];

    if public_paths.iter().any(|&s| uri.contains(s)) {
        return Ok(req);
    }

    let Some(header) = req.metadata().get(header::AUTHORIZATION.as_str()) else {
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
            let mut req = req;
            req.extensions_mut().insert(claim);
            Ok(req)
        }
        Err(_) => {
            let error = Err(Status::unauthenticated("invalid authz token"));
            error!("{error:?}");
            error
        }
    }
}
