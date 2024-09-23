//! An authenticator for our admin endpoints. Technically should work for any Basic auth
//! with some very minor modifications, but that's all we use it for now.
use std::convert::Infallible;

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2, PasswordHash, PasswordVerifier,
};
use axum::response::IntoResponse;
use base64::prelude::*;
use hyper::StatusCode;
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use thiserror::Error;
use tracing::error;

use super::CredentialVerifier;

/// Errors that can occur while verifying authentication for the admin REST endpoints
#[derive(Error, Debug)]
#[allow(missing_docs)]
pub enum AdminAuthError {
    #[error("basic auth: could not retrieve secret fron environment: {0}")]
    EnvError(#[from] envy::Error),
    #[error("basic auth: an error occurred while hashing the password")]
    PasswordHashError,
    #[error("basic auth: this expected basic authentication, but another type was used")]
    WrongAuthType,
    #[error("basic auth: the auth type was correct, but the header was malformed")]
    MalformedAuth,
    #[error("{0}")]
    GenericMessage(String),
}

impl IntoResponse for AdminAuthError {
    fn into_response(self) -> axum::response::Response {
        match self {
            AdminAuthError::EnvError(_) | AdminAuthError::PasswordHashError => {
                (StatusCode::INTERNAL_SERVER_ERROR, "internal server error").into_response()
            }
            AdminAuthError::WrongAuthType
            | AdminAuthError::MalformedAuth
            | Self::GenericMessage(_) => {
                (StatusCode::UNAUTHORIZED, "Basic realm = \"admin\"").into_response()
            }
        }
    }
}

impl From<argon2::password_hash::Error> for AdminAuthError {
    fn from(_: argon2::password_hash::Error) -> Self {
        Self::PasswordHashError
    }
}

#[derive(Clone)]
/// Authenticates the admin REST endpoints, works in principle for any Basic auth,
/// though you'd need to modify it to pass in the `realm` for the errors.
pub struct AdminAuthVerifier {
    /// Our hashing algorithm
    algo: Argon2<'static>,
    /// The hashed, base64 auth password
    hashed: SecretString,
}

impl AdminAuthVerifier {
    /// Creates a new [`AdminAuthVerifier`] from the secrets set in environment variables.
    pub fn from_env() -> Result<Self, AdminAuthError> {
        let config = AdminAuthConfig::from_env()?;
        let encoded = config.into_base64();

        let salt = SaltString::generate(&mut OsRng);

        let algo = Argon2::default();
        let hashed = SecretString::new(
            algo.hash_password(encoded.expose_secret().as_bytes(), &salt)
                .inspect_err(|e| error!("error hashing env password {e}"))?
                .to_string(),
        );

        Ok(Self { algo, hashed })
    }
}

impl CredentialVerifier for AdminAuthVerifier {
    // We don't pass anything like a claim, so we just use an unconstructable type
    // if `!` ever gets stabilized, switch to that.
    type Extension = Infallible;

    type Error = AdminAuthError;

    fn verify(
        &self,
        credential: &axum::http::HeaderValue,
    ) -> Result<Option<Self::Extension>, Self::Error> {
        let mut credential = credential.to_str().unwrap_or("").split_ascii_whitespace();

        if credential.next().filter(|v| *v == "Basic").is_none() {
            return Err(AdminAuthError::WrongAuthType);
        }

        if let Some(credential) = credential.next() {
            let hash =
                PasswordHash::new(self.hashed.expose_secret()).expect("password hash is broken");
            self.algo.verify_password(credential.as_bytes(), &hash)?;

            Ok(None)
        } else {
            Err(AdminAuthError::MalformedAuth)
        }
    }

    fn unauthorized(&self, message: &str) -> Self::Error {
        AdminAuthError::GenericMessage(message.to_owned())
    }
}

/// A config that parses admin secrets from environment variables and then shreds them when done.
#[derive(Deserialize)]
pub struct AdminAuthConfig {
    /// The admin's username
    admin_user: SecretString,
    /// The admin's password
    admin_password: SecretString,
}

impl AdminAuthConfig {
    /// Loads the secret from the environment
    pub fn from_env() -> Result<AdminAuthConfig, envy::Error> {
        dotenvy::dotenv().ok();

        envy::prefixed("APP_").from_env::<AdminAuthConfig>()
    }

    /// Converts this into a [`base64`] encoded secret. This *ideally* will
    /// shred all intermediate data, but that can never be guaranteed. It tries its best,
    /// though.
    fn into_base64(self) -> SecretString {
        let mut secret = String::with_capacity(
            self.admin_password.expose_secret().len() + 1 + self.admin_user.expose_secret().len(),
        );

        secret.push_str(self.admin_user.expose_secret());
        secret.push(':');
        secret.push_str(self.admin_password.expose_secret());
        let secret = SecretString::new(secret);

        SecretString::new(BASE64_STANDARD.encode(secret.expose_secret()))
    }

    /// Gets the inner values for other uses, mostly for tests
    #[allow(dead_code)]
    pub fn into_inner(self) -> (SecretString, SecretString) {
        (self.admin_user, self.admin_password)
    }
}
