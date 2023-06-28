use http::header;
use tonic::{transport::server::Router, Request, Status};
use tonic_reflection::server::ServerReflection;
use tracing::info;

use crate::feature::register;

use super::infrastructure::Infrastructure;

#[tracing::instrument]
pub fn require_auth(req: Request<()>) -> Result<Request<()>, Status> {
    info!("validating request authorization");

    let Some(token) = req.metadata().get(header::AUTHORIZATION.as_str()) else {
        return Err(Status::unauthenticated("missing authz header"))
    };

    let token = token.to_str().unwrap_or("");

    if token.len() == crate::feature::register::TOKEN_LENGTH {
        Ok(req)
    } else {
        Err(Status::unauthenticated("invalid authz token"))
    }
}

pub fn build_reflection_service(
) -> tonic_reflection::server::ServerReflectionServer<impl ServerReflection> {
    let file_descriptor_set = tonic::include_file_descriptor_set!("ratings_descriptor");

    tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(file_descriptor_set)
        .build()
        .unwrap()
}

pub fn build_public_servers<R>(
    router: Router<R>,
    infra: std::sync::Arc<Infrastructure>,
) -> Router<R> {
    let register_service = register::service::build_service(infra);

    router.add_service(register_service)
}

pub fn build_private_servers<R>(
    router: Router<R>,
    infra: std::sync::Arc<Infrastructure>,
) -> Router<R> {
    router
}
