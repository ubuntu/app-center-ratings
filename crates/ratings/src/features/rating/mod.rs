//! Contains various feature implementations for retrieving snap ratings.
//!
//! [`AppService`]: service::AppService

pub use service::RatingService;

mod errors;
pub mod infrastructure;
mod service;
mod use_cases;
