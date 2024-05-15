//! Provides functions for initializing and "warming up" the database, such as setting snap categories.
#![allow(dead_code, unused_imports)]

use snapd::api::convenience::SnapNameFromId;
use snapd::api::find::FindSnapByName;
use snapd::{SnapdClient, SnapdClientError};
use sqlx::{Acquire, Executor, Row};
use thiserror::Error;
use tracing::error;

use crate::app::AppContext;
use crate::features::pb::chart::Category;
use crate::utils::{Infrastructure, Migrator};

/// Errors that can happen while warming up the database
#[derive(Error, Debug)]
pub enum WarmupError {
    /// Errors from `snapd-rs`
    #[error("an error occurred when calling snapd: {0}")]
    SnapdError(#[from] SnapdClientError),
    /// An error that occurred in category updating
    #[error("an error occurred with the DB when getting categories: {0}")]
    CategoryDBError(#[from] sqlx::Error),
    /// An unknown error occurred
    #[error("an unknown error occurred")]
    Unknown,
}

/// Convenience function for getting categories by their snap ID, since it takes multiple API calls
pub(crate) async fn snapd_categories_by_snap_id(
    client: &SnapdClient,
    snap_id: &str,
) -> Result<Vec<Category>, WarmupError> {
    let snap_name = SnapNameFromId::get_name(snap_id.into(), client).await?;

    Ok(FindSnapByName::get_categories(snap_name, client)
        .await?
        .into_iter()
        .map(|v| Category::try_from(v.name.as_ref()).expect("got unknown category?"))
        .collect())
}

/// Update the category (we do this every time we get a vote for the time being)
pub(crate) async fn update_category(
    app_ctx: &AppContext,
    snap_id: &str,
) -> Result<(), WarmupError> {
    let mut pool = app_ctx
        .infrastructure()
        .repository()
        .await
        .map_err(|error| {
            error!("{error:?}");
            WarmupError::Unknown
        })?;

    let snapd_client = &app_ctx.infrastructure().snapd_client;

    let categories = snapd_categories_by_snap_id(snapd_client, snap_id).await?;

    // Do a transaction because bulk querying doesn't seem to work cleanly
    let mut tx = pool.begin().await?;

    // Reset the categories since we're refreshing all of them
    tx.execute(
        sqlx::query("DELETE FROM snap_categories WHERE snap_categories.snap_id = $1;")
            .bind(snap_id),
    )
    .await?;

    for category in categories.iter() {
        tx.execute(
            sqlx::query("INSERT INTO snap_categories (snap_id, category) VALUES ($1, $2); ")
                .bind(snap_id)
                .bind(category),
        )
        .await?;
    }

    tx.commit().await?;
    Ok(())
}

/// Fetches every snap we've had a vote for.
pub async fn all_snaps(ctx: &AppContext) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut conn = ctx.infrastructure().repository().await?;

    Ok(sqlx::query("SELECT DISTINCT snap_id FROM votes")
        .fetch_all(&mut *conn)
        .await?
        .into_iter()
        .map(|v| v.get("snap_id"))
        .collect())
}

/// Categorizes every snap in the database
pub async fn categorize(
    snaps: Vec<String>,
    ctx: &AppContext,
) -> Result<(), Box<dyn std::error::Error>> {
    for snap in snaps {
        let _ = update_category(ctx, &snap)
            .await
            .inspect_err(|e| error!("error updating snap category: {e}"));
    }

    Ok(())
}

/// Performs a full warmup
pub async fn warmup(ctx: &AppContext) -> Result<(), Box<dyn std::error::Error>> {
    let snaps = all_snaps(&ctx).await?;
    categorize(snaps, &ctx).await
}

#[cfg(test)]
mod test {
    use std::collections::HashSet;

    use snapd::SnapdClient;

    use crate::features::pb::chart::Category;

    use super::snapd_categories_by_snap_id;
    const TESTING_SNAP_ID: &str = "3Iwi803Tk3KQwyD6jFiAJdlq8MLgBIoD";
    const TESTING_SNAP_CATEGORIES: [Category; 2] = [Category::Utilities, Category::Development];

    #[tokio::test]
    async fn get_categories() {
        let categories = snapd_categories_by_snap_id(&SnapdClient::default(), TESTING_SNAP_ID)
            .await
            .unwrap();

        assert_eq!(
            TESTING_SNAP_CATEGORIES.into_iter().collect::<HashSet<_>>(),
            categories.into_iter().collect::<HashSet<_>>()
        )
    }
}
