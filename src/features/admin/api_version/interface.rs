//! The public interface of this endpoint

use serde::{Deserialize, Serialize};

use super::ApiVersion;

/// A response serialized as a JSON blob containing the entire branch state
#[derive(Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ApiVersionResponse(pub ApiVersion<'static>);

impl From<ApiVersion<'static>> for ApiVersionResponse {
    fn from(value: ApiVersion<'static>) -> Self {
        ApiVersionResponse(value)
    }
}
