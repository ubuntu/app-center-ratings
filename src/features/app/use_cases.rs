use crate::{
    app::AppContext,
    features::{
        app::{errors::AppError, infrastructure::get_votes_by_snap_id},
        common::entities::Rating,
    },
};
use tracing::error;

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
