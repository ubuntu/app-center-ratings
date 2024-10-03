//! Contains various feature implementations for autenticating users and registering their snap votes.

pub mod entities;

pub use service::UserService;

mod errors;
pub mod infrastructure;
pub mod service;
pub mod use_cases;
