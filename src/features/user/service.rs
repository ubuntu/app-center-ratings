//! Definitions and utilities for building the [`UserService`] for handling [`User`] data.
//!
//! [`User`]: crate::features::user::entities::User
use crate::features::pb::user::user_server::UserServer;

/// An empty struct used to construct a [`UserServer`]
#[derive(Debug, Default)]
pub struct UserService;

/// Builds a new [`UserServer`] with default parameters.
pub fn build_service() -> UserServer<UserService> {
    let service = UserService;
    UserServer::new(service)
}
