//! Business logic building on top of the db layer
mod categories;
mod charts;
mod rating;

use cached::proc_macro::cached;
pub use categories::update_categories;
pub use charts::{Chart, ChartData};
pub use rating::{calculate_band, Rating, RatingsBand};
use serde::{de::DeserializeOwned, Deserialize};

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

#[inline]
async fn get_json<T: DeserializeOwned>(
    url: reqwest::Url,
    query: &[(&str, &str)],
    client: &reqwest::Client,
) -> Result<T, Error> {
    let s = client
        .get(url)
        .header("User-Agent", "ratings-service")
        .header("Snap-Device-Series", 16)
        .query(query)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;

    Ok(serde_json::from_str(&s)?)
}

#[cfg_attr(
    not(feature = "skip_cache"),
    cached(
        key = "String",
        convert = r##"{String::from(snap_id)}"##,
        result = true
    )
)]
async fn get_snap_name(
    snap_id: &str,
    base_url: &reqwest::Url,
    client: &reqwest::Client,
) -> Result<String, Error> {
    let assertions_url = base_url
        .join(&format!("assertions/snap-declaration/16/{snap_id}"))
        .map_err(|e| Error::InvalidUrl(e.to_string()))?;

    let AssertionsResp {
        headers: Headers { snap_name },
    } = get_json(assertions_url, &[], client).await?;

    return Ok(snap_name);

    // serde structs
    //
    #[derive(Debug, Deserialize)]
    struct AssertionsResp {
        headers: Headers,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "kebab-case")]
    struct Headers {
        snap_name: String,
    }
}
