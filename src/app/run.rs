//! Contains definitions for runningi the app context.
use std::net::SocketAddr;

use tower::ServiceBuilder;
use tracing::info;

use crate::{
    app::{
        context::AppContext,
        interfaces::{middleware::ContextMiddlewareLayer, servers::GrpcServiceBuilder},
    },
    utils::{Config, Infrastructure, Migrator},
};

/// Runs the app given the associated [`Config`].
pub async fn run(config: Config) -> Result<(), Box<dyn std::error::Error>> {
    let migrator = Migrator::new(&config.migration_postgres_uri).await?;
    migrator.run().await?;
    let infra = Infrastructure::new(&config).await?;
    let app_ctx = AppContext::new(&config, infra);

    let service = ServiceBuilder::new()
        .layer(ContextMiddlewareLayer::new(app_ctx))
        .service(GrpcServiceBuilder::default().build());

    let socket: SocketAddr = config.socket().parse()?;
    info!("Binding to {socket}");
    hyper::Server::bind(&socket)
        .serve(tower::make::Shared::new(service))
        .await?;
    Ok(())
}
