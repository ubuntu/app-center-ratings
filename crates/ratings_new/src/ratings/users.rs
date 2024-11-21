// FIXME: Remove these dependencies
use sqlx::PgConnection;

use crate::{db::{user::User, vote::Vote}, Context};
use super::Error;

/// Create a [`User`] entry, or note that the user has recently been seen, within the current
/// [`AppContext`].
pub async fn create_or_seen_user(
    ctx: &Context,
    user: User,
    conn: &mut PgConnection,
) -> Result<User, Error> {
    todo!()
}

/// Deletes a [`User`] with the given [`ClientHash`]
///
/// [`ClientHash`]: crate::features::user::entities::ClientHash
pub async fn delete_user_by_client_hash(
    ctx: &Context,
    client_hash: &str,
    conn: &mut PgConnection,
) -> Result<u64, Error> {
    todo!()
}

/// Saves a [`Vote`] to the database, if possible.
pub async fn save_vote_to_db(
    app_ctx: &Context,
    vote: Vote,
    conn: &mut PgConnection,
) -> Result<u64, Error> {
    let _ = vote;
    todo!()
}

/// Retrieve all votes for a given [`User`], within the current [`AppContext`].
///
/// May be filtered for a given snap ID.
pub async fn find_user_votes(
    ctx: &Context,
    client_hash: String,
    snap_id_filter: Option<String>,
    conn: &mut PgConnection,
) -> Result<Vec<Vote>, Error> {
    todo!()
}

/// Gets votes for a snap with the given ID from a given [`ClientHash`]
///
/// [`ClientHash`]: crate::features::user::entities::ClientHash
pub async fn get_snap_votes_by_client_hash(
    ctx: &Context,
    snap_id: String,
    client_hash: String,
    conn: &mut PgConnection,
) -> Result<Vec<Vote>, Error> {
    todo!()
}
