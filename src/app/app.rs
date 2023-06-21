use std::net::SocketAddr;

use axum::Router;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

use crate::{admin::build_admin_router, utils, vote::build_vote_router};

pub async fn build_and_run_server() {
    // a builder for `FmtSubscriber`.
    let subscriber = FmtSubscriber::builder()
        // all spans/events with a level higher than TRACE (e.g, debug, info, warn, etc.)
        // will be written to stdout.
        .with_max_level(Level::TRACE)
        // completes the builder.
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let port = utils::env::get_port();
    let address = utils::env::get_address();

    let socket: SocketAddr = format!("{address}:{port}").parse().unwrap();

    info!("listening on {}", socket);

    let routes = Router::new()
        .merge(build_admin_router())
        .merge(build_vote_router());

    axum::Server::bind(&socket)
        .serve(routes.into_make_service())
        .await
        .unwrap();
}
