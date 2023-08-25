use crate::app::AppContext;
use tracing::error;

use super::{entities::Rating, errors::AppError, infrastructure::get_votes_by_snap_id};

pub async fn get_rating(app_ctx: &AppContext, snap_id: String) -> Result<Rating, AppError> {
    let votes = get_votes_by_snap_id(app_ctx, &snap_id)
        .await
        .map_err(|error| {
            error!("{error:?}");
            AppError::Unknown
        })?;

    let rating = Rating::new(snap_id, votes);

    return Ok(rating);
}
