//! Contains generation and definitions for the [`AppService`]
use crate::features::pb::app::app_server::AppServer;

/// The general service governing retrieving ratings for the store app.
#[derive(Debug, Default)]
pub struct AppService;

/// Builds a new [`AppServer`] using the given [`AppService`] with default parameters.
pub fn build_service() -> AppServer<AppService> {
    let service = AppService;
    AppServer::new(service)
}
