//! Contains definitions for service routers and related components.

use tonic::transport::server::Router;
use tonic_reflection::server::{ServerReflection, ServerReflectionServer};

use crate::features::{chart, get_rating, user};

/// Creates a new default reflection server for this app
pub fn build_reflection_service() -> ServerReflectionServer<impl ServerReflection> {
    let file_descriptor_set = tonic::include_file_descriptor_set!("ratings_descriptor");

    tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(file_descriptor_set)
        .build()
        .unwrap()
}

/// Registers new services required to make the passed in [`Router`] work,
/// the [`Router`] won't be otherwise modified.
pub fn build_servers<R>(router: Router<R>) -> Router<R> {
    let user_service = user::service::build_service();
    let app_service = get_rating::service::build_service();
    let chart_service = chart::service::build_service();

    router
        .add_service(user_service)
        .add_service(app_service)
        .add_service(chart_service)
}
