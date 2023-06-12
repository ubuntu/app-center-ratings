use std::net::SocketAddr;

use axum::{response::Html, routing::get, Router, ServiceExt};
use mongodb::{options::ClientOptions, Client};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("here we go!");
    let mut client_options = ClientOptions::parse("mongodb://root:covfefe@localhost:27017").await?;
    let client = Client::with_options(client_options)?;

    let router = Router::new().route("/", get(|| async { Html("Hello <strong>you!</strong>") }));

    let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
    axum::Server::bind(&addr)
        .serve(router.into_make_service())
        .await
        .unwrap();

    // for db_name in client.list_database_names(None, None).await? {
    //     println!("{}", db_name);
    // }

    Ok(())
}
