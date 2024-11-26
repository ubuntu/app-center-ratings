use crate::{db, jwt::JwtVerifier, middleware::AuthLayer, Context};
use std::{fs::read_to_string, net::SocketAddr};
use tonic::{
    transport::{Identity, Server, ServerTlsConfig},
    Status,
};

mod app;
mod charts;
mod user;

use app::RatingService;
use charts::ChartService;
use tracing::warn;
use user::UserService;

impl From<db::Error> for Status {
    fn from(value: db::Error) -> Self {
        Status::internal(value.to_string())
    }
}

pub async fn run_server(ctx: Context) -> Result<(), Box<dyn std::error::Error>> {
    let verifier = JwtVerifier::from_secret(&ctx.config.jwt_secret)?;
    let addr: SocketAddr = ctx.config.socket().parse()?;

    let keychain_path = ctx.config.tls_keychain_path.clone();
    let key_path = ctx.config.tls_key_path.clone();

    let builder = match (keychain_path, key_path) {
        (Some(keychain_path), Some(key_path)) => {
            let keychain = read_to_string(keychain_path)?;
            let key = read_to_string(key_path)?;
            let identity = Identity::from_pem(keychain, key);
            Server::builder().tls_config(ServerTlsConfig::new().identity(identity))?
        }
        (Some(_), None) | (None, Some(_)) => {
            panic!("Both TLS keychain and private key must be provided, or neither.");
        }
        (None, None) => {
            warn!("TLS is not configured as the environment variables are not set.");
            Server::builder()
        }
    };

    builder
        .layer(AuthLayer::new(verifier))
        .add_service(RatingService::new_server())
        .add_service(ChartService::new_server())
        .add_service(UserService::new_server(ctx))
        .serve(addr)
        .await?;

    Ok(())
}
