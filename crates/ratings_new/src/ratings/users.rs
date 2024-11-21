// FIXME: Remove these dependencies
use sqlx::PgConnection;

use super::Error;
use crate::{
    db::{user::User, vote::Vote},
    Context,
};

/// Create a [`User`] entry, or note that the user has recently been seen, within the current
/// [`AppContext`].
pub async fn create_or_seen_user(
    user: User,
    ctx: &Context,
    conn: &mut PgConnection,
) -> Result<User, Error> {
    todo!()
}

/// Deletes a [`User`] with the given [`ClientHash`]
///
/// [`ClientHash`]: crate::features::user::entities::ClientHash
pub async fn delete_user_by_client_hash(
    client_hash: &str,
    ctx: &Context,
    conn: &mut PgConnection,
) -> Result<u64, Error> {
    todo!()
}

/// Saves a [`Vote`] to the database, if possible.
pub async fn save_vote_to_db(
    vote: Vote,
    app_ctx: &Context,
    conn: &mut PgConnection,
) -> Result<u64, Error> {
    let _ = vote;
    todo!()
}

/// Retrieve all votes for a given [`User`], within the current [`AppContext`].
///
/// May be filtered for a given snap ID.
pub async fn find_user_votes(
    client_hash: String,
    snap_id_filter: Option<String>,
    ctx: &Context,
    conn: &mut PgConnection,
) -> Result<Vec<Vote>, Error> {
    todo!()
}

/// Gets votes for a snap with the given ID from a given [`ClientHash`]
///
/// [`ClientHash`]: crate::features::user::entities::ClientHash
pub async fn get_snap_votes_by_client_hash(
    snap_id: String,
    client_hash: String,
    ctx: &Context,
    conn: &mut PgConnection,
) -> Result<Vec<Vote>, Error> {
    todo!()
}
