//! The interface between the outside world

use std::{convert::Infallible, str::FromStr};

use crate::app::AppContext;

use super::super::interface::{GetLogLevelResponse, SetLogLevelRequest, SetLogLevelResponse};

use axum::extract;
use log::Level;
use tracing::level_filters::LevelFilter;

/// Converts the log level into the proper representation for use with business-logic methods
pub async fn set_log_level(
    extract::Extension(app_context): extract::Extension<AppContext>,
    extract::Json(req): extract::Json<SetLogLevelRequest>,
) -> Result<extract::Json<SetLogLevelResponse>, Infallible> {
    let level = match req.level {
        log::Level::Error => LevelFilter::ERROR,
        log::Level::Warn => LevelFilter::WARN,
        log::Level::Info => LevelFilter::INFO,
        log::Level::Debug => LevelFilter::DEBUG,
        log::Level::Trace => LevelFilter::TRACE,
    };

    super::super::set_log_level(&app_context.infrastructure().log_reload_handle, level);

    Ok(SetLogLevelResponse.into())
}

/// Retrieves the log level, converting it into the proper response for [`axum`].
pub async fn get_log_level(
    extract::Extension(app_context): extract::Extension<AppContext>,
) -> Result<extract::Json<GetLogLevelResponse>, Infallible> {
    let level = super::super::get_log_level(&app_context.infrastructure().log_reload_handle);

    Ok(GetLogLevelResponse {
        level: Level::from_str(level.into_level().unwrap().as_str()).unwrap(),
    }
    .into())
}
