//! Definitions and utilities for building the [`ChartService`] for using the [`Chart`] feature.
//!
//! [`Chart`]: crate::features::chart::entities::Chart

use crate::features::pb::chart::chart_server::ChartServer;

/// An empty struct denoting that allows the building of a [`ChartServer`].
#[derive(Debug, Default)]
pub struct ChartService;

/// Creates a [`ChartServer`] with default barameters from a [`ChartService`].
pub fn build_service() -> ChartServer<ChartService> {
    let service = ChartService;
    ChartServer::new(service)
}
