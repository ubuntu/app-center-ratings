use http::header;
use tonic::{Request, Status};

use crate::app::context::Context;

pub fn authentication(req: Request<()>) -> Result<Request<()>, Status> {
    let ctx = req.extensions().get::<Context>();

    if ctx.is_none() {
        return Err(Status::unknown("no request context"));
    }
    let ctx = ctx.unwrap();
    let uri = &ctx.uri;

    if uri.contains("ratings.features.user.User/Login")
        || uri.contains("grpc.reflection.v1alpha.ServerReflection/ServerReflectionInfo")
    {
        return Ok(req);
    }

    let Some(token) = req.metadata().get(header::AUTHORIZATION.as_str()) else {
        return Err(Status::unauthenticated("missing authz header"));
    };

    let token = token.to_str().unwrap_or("");

    if token.len() == crate::features::user::TOKEN_LENGTH {
        Ok(req)
    } else {
        Err(Status::unauthenticated("invalid authz token"))
    }
}
