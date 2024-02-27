//! Contains generation and definitions for the [`AppService`]
use crate::features::pb::app::app_server::AppServer;

mod grpc;

/// The general service governing retrieving ratings for the store app.
#[derive(Copy, Clone, Debug, Default)]
pub struct RatingService;

impl RatingService {
    /// Converts this service into its corresponding server
    pub fn to_server(self) -> AppServer<RatingService> {
        self.into()
    }
}

impl From<RatingService> for AppServer<RatingService> {
    fn from(value: RatingService) -> Self {
        AppServer::new(value)
    }
}
