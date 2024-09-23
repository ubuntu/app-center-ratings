//! Contains API endpoints for manipulating the log level

use tracing::level_filters::LevelFilter;
use tracing_subscriber::{reload::Handle, Registry};

pub mod interface;
pub mod service;

/// Replaces the current app's global log level with the given level filter.
pub fn set_log_level(reload_handle: &Handle<LevelFilter, Registry>, level: LevelFilter) {
    reload_handle
        .modify(|layer| *layer = level)
        .expect("setting global log level not working");

    tracing::info!("log level changed to \"{}\"", level);
}

/// Retrieves the current log level from the application
pub fn get_log_level(reload_handle: &Handle<LevelFilter, Registry>) -> LevelFilter {
    let mut level = None;
    reload_handle
        .modify(|layer| level = Some(*layer))
        .expect("getting global log level not working");

    level.unwrap()
}
