//! Contains the service definitions for the API version functionality

use axum::{routing::get, Router};

mod rest;
use rest::get_api_version;

/// The route we want to service
const ROUTE: &str = "/admin/api-version";

/// Essentially a builder for the API route registration
pub struct ApiVersionService;

impl ApiVersionService {
    /// Registers the route with axum
    pub fn register_axum_route(self) -> Router {
        Router::new().route(ROUTE, get(get_api_version))
    }
}
