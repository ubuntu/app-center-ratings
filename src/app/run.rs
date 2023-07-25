use std::{net::SocketAddr, time::Duration};

use tonic::transport::Server;
use tower::ServiceBuilder;
use tracing::info;

use crate::utils;

use super::interfaces::routes::{build_reflection_service, build_servers};
use super::interfaces::{authentication::authentication, middleware::ContextMiddlewareLayer};

pub async fn run() {
    let layer = ServiceBuilder::new()
        .timeout(Duration::from_secs(30))
        .layer(ContextMiddlewareLayer::default())
        .layer(tonic::service::interceptor(authentication))
        .into_inner();

    let server = Server::builder()
        .layer(layer)
        .add_service(build_reflection_service());

    let server = build_servers(server);

    let socket: SocketAddr = utils::env::get_socket().parse().unwrap();
    info!("Binding to {socket}");
    server.serve(socket).await.unwrap();
}
