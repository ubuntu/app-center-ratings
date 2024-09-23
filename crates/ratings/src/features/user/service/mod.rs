//! Definitions and utilities for building the [`UserService`] for handling [`User`] data.
//!
//! [`User`]: crate::features::user::entities::User
use crate::features::pb::user::user_server::UserServer;

mod grpc;

/// An empty struct used to construct a [`UserServer`]
#[derive(Copy, Clone, Debug, Default)]
pub struct UserService;

impl UserService {
    /// The paths which are accessible without authentication, if any
    pub const PUBLIC_PATHS: [&'static str; 2] = [
        "ratings.features.user.User/Register",
        "ratings.features.user.User/Authenticate",
    ];

    /// Converts this service into its corresponding server
    pub fn to_server(self) -> UserServer<UserService> {
        self.into()
    }
}

impl From<UserService> for UserServer<UserService> {
    fn from(value: UserService) -> Self {
        UserServer::new(value)
    }
}
