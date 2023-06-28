use std::sync::Arc;
use std::{net::SocketAddr, time::Duration};
use tonic::transport::Server;
use tower::ServiceBuilder;
use tracing::info;

use super::infrastructure::Infrastructure;

use super::interfaces::{build_private_servers, build_public_servers, build_reflection_service};
use crate::utils;

pub async fn build_and_run() {
    let layer = ServiceBuilder::new()
        .timeout(Duration::from_secs(30))
        .into_inner();

    let infra = Infrastructure::new().await;
    let infra = Arc::new(infra);

    let server = Server::builder()
        .layer(layer)
        .add_service(build_reflection_service());

    let server = build_public_servers(server, infra.clone());
    let server = build_private_servers(server, infra.clone());

    let socket: SocketAddr = utils::env::get_socket().parse().unwrap();
    info!("Binding to {socket}");
    server.serve(socket).await.unwrap();
}
