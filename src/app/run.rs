//! Contains definitions for runningi the app context.
use std::convert::Infallible;
use std::{net::SocketAddr, time::Duration};

use tokio::task::{self, JoinError, JoinHandle};
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

/// Allow the warmup watchdog for refreshing stale date run once daily
fn background_refresh(app_ctx: AppContext) -> JoinHandle<Result<Infallible, JoinError>> {

    let regular_warmup = task::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(60 * 60 * 24 /* once a day */));

        loop {
            interval.tick().await;
            let _ = warmup::warmup(&app_ctx).await.inspect_err(|e| tracing::error!("refreshing stage category data resulted in an error: {e}"));
        }
    });

    tokio::task::spawn(regular_warmup)
}   

/// Runs the app given the associated [`Config`].
pub async fn run(config: Config) -> Result<(), Box<dyn std::error::Error>> {
    let migrator = Migrator::new(&config.migration_postgres_uri).await?;
    migrator.run().await?;
    let infra = Infrastructure::new(&config).await?;
    let app_ctx = AppContext::new(&config, infra);

    tracing::info!("Now fetching categories for every snap in the DB, this could take a while...");
    // Fetch all the categories
    warmup::warmup(&app_ctx).await?;

    let background_refresh_handle = background_refresh(app_ctx.clone());

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
    background_refresh_handle.await??;
    Ok(())
}
