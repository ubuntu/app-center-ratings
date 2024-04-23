//! The interface between the outside world

use std::convert::Infallible;

use super::super::{interface::ApiVersionResponse, ApiVersion};

use axum::extract;

/// Converts the API version into the proper representation for use with business-logic methods
pub async fn get_api_version() -> Result<extract::Json<ApiVersionResponse>, Infallible> {
    Ok(ApiVersionResponse(ApiVersion::build_info()).into())
}
