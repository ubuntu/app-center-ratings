//! Contains various feature implementations for autenticating users and registering their snap votes.

pub mod entities;

pub use service::UserService;

pub(crate) mod infrastructure;

mod errors;
mod service;
mod use_cases;
