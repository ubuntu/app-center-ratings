use std::{net::SocketAddr, time::Duration};

use tonic::transport::Server;
use tower::ServiceBuilder;
use tracing::info;

use crate::{
    app::context::AppContext,
    app::interfaces::{
        authentication::authentication,
        middleware::ContextMiddlewareLayer,
        routes::{build_reflection_service, build_servers},
    },
    utils::{Config, Infrastructure, Migrator},
};

pub async fn run(config: Config) -> Result<(), Box<dyn std::error::Error>> {
    let migrator = Migrator::new(&config.migration_postgres_uri).await?;
    migrator.run().await?;
    let infra = Infrastructure::new(&config).await?;
    let app_ctx = AppContext::new(&config, infra);

    let layer = ServiceBuilder::new()
        .timeout(Duration::from_secs(30))
        .layer(ContextMiddlewareLayer::new(app_ctx))
        .layer(tonic::service::interceptor(authentication))
        .into_inner();

    let server = Server::builder()
        .layer(layer)
        .add_service(build_reflection_service());
    let server = build_servers(server);

    let socket: SocketAddr = config.socket().parse()?;
    info!("Binding to {socket}");
    server.serve(socket).await?;

    Ok(())
}
