//! Updating snap categories from data in snapcraft.io
use crate::{
    db::{set_categories_for_snap, snap_has_categories, Category},
    ratings::{get_json, get_snap_name, Error},
    Context,
};
use serde::Deserialize;
use sqlx::PgConnection;
use std::sync::Arc;
use tokio::sync::Notify;
use tracing::error;

/// Update the categories for a given snap.
///
/// In the case where we do not have categories, we need to fetch them and store them in the DB.
/// This is racey without coordination so we check to see if any other tasks are currently attempting
/// this and block on them completing if they are, if not then we set up the Notify and they block on us.
pub async fn update_categories(
    snap_id: &str,
    ctx: &Context,
    conn: &mut PgConnection,
) -> Result<(), Error> {
    // If we have categories for the requested snap in place already then skip updating.
    // Eventually we will need to update and refresh categories over time but the assumption for now is
    // that snap categories do not change frequently so we do not need to eagerly update them.
    if snap_has_categories(snap_id, conn).await? {
        return Ok(());
    }

    let mut guard = ctx.category_updates.lock().await;
    let (notifier, should_wait) = match guard.get(&snap_id.to_string()) {
        Some(notifier) => (notifier.clone(), true),
        None => (Arc::new(Notify::new()), false),
    };

    if should_wait {
        // Another task is updating the categories for this snap so wait for it to complete and then
        // return: https://docs.rs/tokio/latest/tokio/sync/struct.Notify.html#method.notified
        drop(guard);
        notifier.notified().await;
        return Ok(());
    }

    // At this point we can release the mutex for other calls to update_categories to proceed while
    // we update the DB state for the snap_id we are interested in. Any calls between now and when
    // we complete the update will block on the notifier we insert here.
    guard.insert(snap_id.to_string(), notifier.clone());
    drop(guard);

    // We can't early return while holding the Notifier as that will leave any waiting tasks
    // blocked. Rather than attempt to retry at this stage we allow for stale category data
    // until a new task attempts to get data for the same snap.
    let base = &ctx.config.snapcraft_io_uri;
    if let Err(e) = update_categories_inner(snap_id, base, &ctx.http_client, conn).await {
        error!(%snap_id, "unable to update snap categories: {e}");
    }

    // Grab the mutex around the category_updates so any incoming tasks block behind us and then
    // notify all blocked tasks before removing the Notify from the map.
    let mut guard = ctx.category_updates.lock().await;
    notifier.notify_waiters();
    guard.remove(&snap_id.to_string());

    Ok(())
}

#[inline]
async fn update_categories_inner(
    snap_id: &str,
    base: &str,
    client: &reqwest::Client,
    conn: &mut PgConnection,
) -> Result<(), Error> {
    let categories = get_snap_categories(snap_id, base, client).await?;
    set_categories_for_snap(snap_id, categories, conn).await?;

    Ok(())
}

/// Pull snap categories by for a given snapd_id from the snapcraft.io rest API
async fn get_snap_categories(
    snap_id: &str,
    base: &str,
    client: &reqwest::Client,
) -> Result<Vec<Category>, Error> {
    let snap_name = get_snap_name(snap_id, base, client).await?;

    let base_url = reqwest::Url::parse(base).map_err(|e| Error::InvalidUrl(e.to_string()))?;
    let info_url = base_url
        .join(&format!("snaps/info/{snap_name}"))
        .map_err(|e| Error::InvalidUrl(e.to_string()))?;

    let FindResp {
        snap: SnapInfo { categories },
    } = get_json(info_url, &[("fields", "categories")], client).await?;

    let res: Result<Vec<Category>, Error> = categories
        .into_iter()
        .map(|c| Category::try_from(c.name.as_str()).map_err(Into::into))
        .collect();

    return res;

    // serde structs

    #[derive(Debug, Deserialize)]
    struct FindResp {
        snap: SnapInfo,
    }

    #[derive(Debug, Deserialize)]
    struct SnapInfo {
        categories: Vec<RawCategory>,
    }

    #[derive(Debug, Deserialize)]
    struct RawCategory {
        name: String,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Can be run explicitly to validate the behaviour of the API calls we make against
    // snapcraft.io but we don't want to do this in local testing or CI by default.
    #[ignore = "hits snapcraft.io"]
    #[tokio::test]
    async fn get_snap_categories_works() {
        let client = reqwest::Client::new();
        let base = "https://api.snapcraft.io/v2/";
        let snap_id = "NeoQngJVBf2wKC48bxnF2xqmfEFGdVnx"; // steam
        let categories = get_snap_categories(snap_id, base, &client).await.unwrap();

        assert_eq!(categories, vec![Category::Games]);
    }
}
