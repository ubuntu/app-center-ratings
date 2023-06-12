use std::net::SocketAddr;

use axum::{routing::get, Router};

use crate::utils;

pub async fn build_and_run_server() {
    let port = utils::env::get_port();
    let address = utils::env::get_address();

    let socket: SocketAddr = format!("{address}:{port}").parse().unwrap();

    let routes = Router::new().route("/", get(health));

    axum::Server::bind(&socket)
        .serve(routes.into_make_service())
        .await
        .unwrap();
}

async fn health() -> &'static str {
    "OK"
}
