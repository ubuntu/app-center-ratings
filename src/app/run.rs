//! Contains definitions for runningi the app context.
use std::{net::SocketAddr, time::Duration};

use tower::ServiceBuilder;
use tracing::info;

use crate::utils::warmup;

use crate::{
    app::{
        context::AppContext,
        interfaces::{middleware::ContextMiddlewareLayer, servers::AppCenterRatingsService},
    },
    utils::{Config, Infrastructure, Migrator},
};

/// Runs the app given the associated [`Config`].
pub async fn run(config: Config) -> Result<(), Box<dyn std::error::Error>> {
    let migrator = Migrator::new(&config.migration_postgres_uri).await?;
    migrator.run().await?;
    let infra = Infrastructure::new(&config).await?;
    let app_ctx = AppContext::new(&config, infra);

    tracing::info!("Now fetching categories for every snap in the DB, this could take a while...");
    // Fetch all the categories
    warmup::warmup(&app_ctx).await?;

    info!("{} infrastructure initialized", config.name);

    let socket: SocketAddr = config.socket().parse()?;
    // Shred the secrets in `config`
    drop(config);

    let service = ServiceBuilder::new()
        .timeout(Duration::from_secs(30))
        .layer(ContextMiddlewareLayer::new(app_ctx))
        .service(AppCenterRatingsService::with_default_routes());

    let shared = tower::make::Shared::new(service);

    info!("Binding to {socket}");
    hyper::Server::bind(&socket).serve(shared).await?;
    Ok(())
}
