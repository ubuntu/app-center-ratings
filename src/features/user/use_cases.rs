//! Helper functions to call things related to [`User`] handling.

use tracing::warn;

use crate::{
    app::AppContext,
    features::user::infrastructure::{find_user_votes, save_vote_to_db},
    features::user::{
        entities::{User, Vote},
        errors::UserError,
        infrastructure::{
            create_or_seen_user, delete_user_by_client_hash, get_snap_votes_by_client_hash,
        },
    },
};

use super::infrastructure::update_category;

/// Create a [`User`] entry, or note that the user has recently been seen, within the current
/// [`AppContext`].
pub async fn authenticate(app_ctx: &AppContext, id: &str) -> Result<User, UserError> {
    let user = User::new(id);
    create_or_seen_user(app_ctx, user).await
}

/// Deletes a [`User`] with the given [`ClientHash`]
///
/// [`ClientHash`]: crate::features::user::entities::ClientHash
pub async fn delete_user(app_ctx: &AppContext, client_hash: &str) -> Result<(), UserError> {
    let result = delete_user_by_client_hash(app_ctx, client_hash).await;
    result?;
    Ok(())
}

/// Saves a [`Vote`] to the database, if possible.
#[allow(unused_must_use)]
pub async fn vote(app_ctx: &AppContext, vote: Vote) -> Result<(), UserError> {
    // Ignore but log warning, it's not fatal
    // update_category(app_ctx, &vote.snap_id)
    //     .await
    //     .inspect_err(|e| warn!("{}", e));
    let result = save_vote_to_db(app_ctx, vote).await;
    result?;
    Ok(())
}

/// Gets votes for a snap with the given ID from a given [`ClientHash`]
///
/// [`ClientHash`]: crate::features::user::entities::ClientHash
#[allow(unused_must_use)]
pub async fn get_snap_votes(
    app_ctx: &AppContext,
    snap_id: String,
    client_hash: String,
) -> Result<Vec<Vote>, UserError> {
    // Ignore but log warning, it's not fatal
    // update_category(app_ctx, &snap_id)
    //     .await
    //     .inspect_err(|e| warn!("{}", e));
    get_snap_votes_by_client_hash(app_ctx, snap_id, client_hash).await
}

/// Retrieve all votes for a given [`User`], within the current [`AppContext`].
///
/// May be filtered for a given snap ID.
pub async fn list_my_votes(
    app_ctx: &AppContext,
    client_hash: String,
    snap_id_filter: Option<String>,
) -> Result<Vec<Vote>, UserError> {
    find_user_votes(app_ctx, client_hash, snap_id_filter).await
}
