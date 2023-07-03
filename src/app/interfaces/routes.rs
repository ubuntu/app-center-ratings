use tonic::transport::server::Router;
use tonic_reflection::server::ServerReflection;

use crate::feature::{register, user};

pub fn build_reflection_service(
) -> tonic_reflection::server::ServerReflectionServer<impl ServerReflection> {
    let file_descriptor_set = tonic::include_file_descriptor_set!("ratings_descriptor");

    tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(file_descriptor_set)
        .build()
        .unwrap()
}

pub fn build_public_servers<R>(router: Router<R>) -> Router<R> {
    let register_service = register::service::build_service();

    router.add_service(register_service)
}

pub fn build_private_servers<R>(router: Router<R>) -> Router<R> {
    let user_service = user::service::build_service();

    router.add_service(user_service)
}
