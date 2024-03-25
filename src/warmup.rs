#![allow(dead_code, unused_imports)]

use sqlx::Row;
use tracing::error;

use app::AppContext;
use features::user::infrastructure;
use utils::{Infrastructure, Migrator};

mod app;
mod features;
mod utils;

async fn all_snaps(ctx: &AppContext) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut conn = ctx.infrastructure().repository().await?;

    Ok(sqlx::query("SELECT DISTINCT snap_id FROM votes")
        .fetch_all(&mut *conn)
        .await?
        .into_iter()
        .map(|v| v.get("snap_id"))
        .collect())
}

async fn categorize(
    snaps: Vec<String>,
    ctx: &AppContext,
) -> Result<(), Box<dyn std::error::Error>> {
    for snap in snaps {
        let _ = infrastructure::update_category(ctx, &snap)
            .await
            .inspect_err(|e| error!("error updating snap category: {e}"));
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = utils::Config::load()?;
    let migrator = Migrator::new(&config.migration_postgres_uri).await?;
    migrator.run().await?;
    let infra = Infrastructure::new(&config).await?;
    let ctx = AppContext::new(&config, infra);

    let snaps = all_snaps(&ctx).await?;
    categorize(snaps, &ctx).await
}
