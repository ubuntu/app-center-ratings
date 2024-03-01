//! Contains the service definitions for the log level functionality

use axum::{
    routing::{get, post},
    Router,
};

mod rest;
use rest::set_log_level;

use self::rest::get_log_level;

/// The route we want to service
const ROUTE: &str = "/admin/log-level";

/// Essentially a builder for registering the log level
pub struct LogLevelService;

impl LogLevelService {
    /// Registers the route with axum
    pub fn register_axum_route(self) -> Router {
        Router::new()
            .route(ROUTE, post(set_log_level))
            .route(ROUTE, get(get_log_level))
    }
}
