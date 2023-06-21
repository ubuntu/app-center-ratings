use axum::Router;
use std::net::SocketAddr;
use tracing::info;

use super::multiplex_service::MultiplexService;

use crate::{
    feature::{
        admin::build_admin_router,
        app::{app_server::AppServer, AppService},
        chart::{chart_server::ChartServer, ChartService},
        user::{user_server::UserServer, UserService},
    },
    utils,
};

pub async fn build_and_run() {
    let socket: SocketAddr = utils::env::get_socket().parse().unwrap();

    let grpc = build_grpc();
    let rest = build_rest();

    // combine them into one service
    let service = MultiplexService::new(rest, grpc);

    info!("Binding to {}", socket);
    axum::Server::bind(&socket)
        .serve(tower::make::Shared::new(service))
        .await
        .unwrap();
}

fn build_rest() -> Router {
    Router::new().merge(build_admin_router())
}

fn build_grpc() -> tonic::transport::server::Routes {
    let file_descriptor_set = tonic::include_file_descriptor_set!("ratings_descriptor");

    let reflection_service = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(file_descriptor_set)
        .build()
        .unwrap();

    let app_service = AppServer::with_interceptor(AppService::default(), super::auth::require_auth);
    let chart_service =
        ChartServer::with_interceptor(ChartService::default(), super::auth::require_auth);
    let user_service =
        UserServer::with_interceptor(UserService::default(), super::auth::require_auth);

    tonic::transport::Server::builder()
        .add_service(reflection_service)
        .add_service(app_service)
        .add_service(chart_service)
        .add_service(user_service)
        .into_service()
}
