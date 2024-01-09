//! Various things you can do with an [`AppService`].
//!
//! [`AppService`]: crate::features::app::service::AppService
use crate::{
    app::AppContext,
    features::{
        app::{errors::AppError, infrastructure::get_votes_by_snap_id},
        common::entities::Rating,
    },
};
use tracing::error;

/// Retrieves votes for the snap indicated by `snap_id` for the given [`AppContext`].
///
/// This will return either a [`VoteSummary`], if successful, or a relevant [`AppError`].
/// The function will fail when failing to retrieve the rating, or if some other unknown network or similar error occurs.
pub async fn get_rating(app_ctx: &AppContext, snap_id: String) -> Result<Rating, AppError> {
    let votes = get_votes_by_snap_id(app_ctx, &snap_id)
        .await
        .map_err(|error| {
            error!("{error:?}");
            AppError::Unknown
        })?;

    let rating = Rating::new(votes);

    Ok(rating)
}
