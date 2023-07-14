use http::header;
use tonic::{Request, Status};

use crate::app::context::Context;
use crate::utils::infrastructure::INFRA;

pub fn authentication(req: Request<()>) -> Result<Request<()>, Status> {
    let ctx = req.extensions().get::<Context>();

    if ctx.is_none() {
        return Err(Status::unknown("no request context"));
    }
    let ctx = ctx.unwrap();
    let uri = &ctx.uri;

    let public_paths = [
        "ratings.features.user.User/Register",
        "ratings.features.user.User/Authenticate",
        "grpc.reflection.v1alpha.ServerReflection/ServerReflectionInfo",
    ];

    if public_paths.iter().any(|&s| uri.contains(s)) {
        return Ok(req);
    }

    let Some(header) = req.metadata().get(header::AUTHORIZATION.as_str()) else {
        return Err(Status::unauthenticated("missing authz header"));
    };

    let raw: Vec<&str> = header.to_str().unwrap_or("").split_whitespace().collect();

    if raw.len() != 2 {
        return Err(Status::unauthenticated("invalid authz token"));
    }

    let token = raw[1];
    let infra = INFRA.get().expect("INFRA should be initialised");
    match infra.jwt.decode(token) {
        Ok(claim) => {
            let mut req = req;
            req.extensions_mut().insert(claim);
            Ok(req)
        }
        Err(_) => Err(Status::unauthenticated("invalid authz token")),
    }
}
