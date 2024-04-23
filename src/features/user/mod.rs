//! Contains various feature implementations for autenticating users and registering their snap votes.

pub mod entities;

pub use service::UserService;

mod errors;
mod infrastructure;
mod service;
mod use_cases;
