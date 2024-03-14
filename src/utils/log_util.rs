//! Allows us to initialize and manipulate the logging framework within our infrastructure

use std::{error::Error, str::FromStr};

use tracing::level_filters::LevelFilter;
use tracing_subscriber::{layer::SubscriberExt, prelude::*, reload::Handle, Registry};

/// Initializes logging app-wide, generating a reload handle for us to use later
pub fn init_logging(log_level: &str) -> Result<Handle<LevelFilter, Registry>, Box<dyn Error>> {
    let (filter, reload_handle) =
        tracing_subscriber::reload::Layer::new(LevelFilter::from_str(log_level)?);

    tracing_subscriber::registry()
        .with(filter)
        .with(
            tracing_subscriber::fmt::layer()
                .with_span_events(tracing_subscriber::fmt::format::FmtSpan::CLOSE),
        )
        .init();

    Ok(reload_handle)
}
