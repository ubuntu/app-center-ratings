//! Business logic building on top of the db layer
mod categories;
mod charts;
mod rating;

pub use categories::update_categories;
pub use charts::{Chart, ChartData};
pub use rating::{calculate_band, Rating, RatingsBand};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Db(#[from] crate::db::Error),

    #[error(transparent)]
    Envy(#[from] envy::Error),

    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),

    #[error(transparent)]
    Strum(#[from] strum::ParseError),

    #[error("invalid url: {0}")]
    InvalidUrl(String),

    #[error(transparent)]
    SnapcraftIo(#[from] reqwest::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),
}
