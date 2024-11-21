use sqlx::PgConnection;
use crate::{db::vote::Vote, Context};
use super::Error;

// FIXME: Remove these dependencies
use ratings::features::{common::entities::VoteSummary, pb::chart::{Category, Timeframe}};

/// Saves a [`Vote`] to the database, if possible.
pub async fn save_vote_to_db(
    app_ctx: &Context,
    vote: Vote,
    conn: &mut PgConnection,
) -> Result<u64, Error> {
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

/// Retrieves votes for the snap indicated by `snap_id` for the given [`AppContext`].
///
/// See the documentation for the common caller, [`get_rating`], for more information.
///
/// [`get_rating`]: crate::features::app::use_cases::get_rating
pub async fn get_votes_by_snap_id(
    app_ctx: &Context,
    snap_id: &str,
    conn: &mut PgConnection,
) -> Result<VoteSummary, Error> {
    todo!()
}

//FIXME: should this live in categories.rs?
/// Retrieves the vote summary in the given [`AppContext`] over a given [`Timeframe`]
/// from the database.
pub async fn get_votes_summary(
    app_ctx: &Context,
    timeframe: Timeframe,
    category: Option<Category>,
    conn: &mut PgConnection,
) -> Result<Vec<VoteSummary>, Error> {
    todo!()
}
