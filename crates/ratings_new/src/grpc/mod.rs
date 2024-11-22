use crate::{db, jwt::JwtVerifier, middleware::AuthLayer, Context};
use std::net::SocketAddr;
use tonic::{transport::Server, Status};

mod app;
mod charts;
mod user;

use app::RatingService;
use charts::ChartService;
use user::UserService;

impl From<db::Error> for Status {
    fn from(value: db::Error) -> Self {
        Status::internal(value.to_string())
    }
}

pub async fn run_server(ctx: Context) -> Result<(), Box<dyn std::error::Error>> {
    let verifier = JwtVerifier::from_secret(&ctx.config.jwt_secret)?;
    let addr: SocketAddr = ctx.config.socket().parse()?;

    Server::builder()
        .layer(AuthLayer::new(verifier))
        .add_service(RatingService::new_server())
        .add_service(ChartService::new_server())
        .add_service(UserService::new_server(ctx))
        .serve(addr)
        .await?;

    Ok(())
}
