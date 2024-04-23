//! Definitions and utilities for building the [`ChartService`] for using the [`Chart`] feature.
//!
//! [`Chart`]: crate::features::chart::entities::Chart

use crate::features::pb::chart::chart_server::ChartServer;

mod grpc;

/// An empty struct denoting that allows the building of a [`ChartServer`].
#[derive(Copy, Clone, Debug, Default)]
pub struct ChartService;

impl ChartService {
    /// The paths which are accessible without authentication, if any
    pub const PUBLIC_PATHS: [&'static str; 0] = [];

    /// Converts this service into its corresponding server
    pub fn to_server(self) -> ChartServer<ChartService> {
        self.into()
    }
}

impl From<ChartService> for ChartServer<ChartService> {
    fn from(value: ChartService) -> Self {
        ChartServer::new(value)
    }
}
