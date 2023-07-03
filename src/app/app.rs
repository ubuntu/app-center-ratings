use std::{net::SocketAddr, time::Duration};
use std::sync::Arc;

use once_cell::sync::OnceCell;
use tonic::transport::Server;
use tower::ServiceBuilder;
use tracing::info;

use crate::utils;
use crate::utils::infrastructure::Infrastructure;

use super::interfaces::{authentication::authentication, middleware::ContextMiddlewareLayer};
use super::interfaces::routes::{
    build_private_servers, build_public_servers, build_reflection_service,
};

pub static INFRA: OnceCell<Infrastructure> = OnceCell::new();

pub async fn build_and_run() {
    let infra = Infrastructure::new().await;
    INFRA.set(infra).expect("infrastructure should have initialised");

    let layer = ServiceBuilder::new()
        .timeout(Duration::from_secs(30))
        .layer(ContextMiddlewareLayer::default())
        .layer(tonic::service::interceptor(authentication))
        .into_inner();


    let server = Server::builder()
        .layer(layer)
        .add_service(build_reflection_service());

    let server = build_public_servers(server);
    let server = build_private_servers(server);

    let socket: SocketAddr = utils::env::get_socket().parse().unwrap();
    info!("Binding to {socket}");
    server.serve(socket).await.unwrap();
}
