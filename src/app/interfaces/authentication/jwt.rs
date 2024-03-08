//! JSON Web Tokens infrastructure and utlities.
use jsonwebtoken::{DecodingKey, Validation};
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use thiserror::Error;
use tracing::error;

use crate::utils::{jwt::Claims, Config};

use super::CredentialVerifier;

/// An error for things that can go wrong with JWT verification
#[derive(Error, Debug)]
#[allow(clippy::missing_docs_in_private_items, missing_docs)]
pub enum JwtVerifierError {
    #[error("jwt: invalid shape")]
    InvalidShape,
    #[error("jwt: could not retrieve secret fron environment: {0}")]
    EnvError(#[from] envy::Error),
    #[error("jwt: error decoding secret: {0}")]
    DecodeSecretError(#[from] jsonwebtoken::errors::Error),
    #[error("jwt: invalid authz token")]
    InvalidHeader,
    #[error(transparent)]
    GenericMessage(#[from] tonic::Status),
}

impl From<JwtVerifierError> for tonic::Status {
    fn from(value: JwtVerifierError) -> Self {
        match value {
            JwtVerifierError::InvalidShape | JwtVerifierError::EnvError(_) => {
                tonic::Status::internal("Internal Server Error")
            }
            JwtVerifierError::DecodeSecretError(_) => {
                tonic::Status::unauthenticated("invalid JWT token")
            }
            JwtVerifierError::InvalidHeader => {
                tonic::Status::unauthenticated("invalid authz header")
            }
            JwtVerifierError::GenericMessage(status) => status,
        }
    }
}

#[derive(Clone)]
/// A JWT verification agent that allows verifying assigned tokens are valid
pub struct JwtVerifier {
    /// A decoding key for receipt
    decoding_key: DecodingKey,
}

impl JwtVerifier {
    /// Attempts to create a new verifier from the invoker's environment.
    pub fn from_env() -> Result<Self, JwtVerifierError> {
        let config = JwtConfig::from_env()?;

        Self::from_secret(&config.jwt_secret)
    }

    /// Creates a new verifier from the given secret.
    pub fn from_secret(secret: &SecretString) -> Result<Self, JwtVerifierError> {
        let decoding_key = DecodingKey::from_base64_secret(secret.expose_secret())?;

        Ok(Self { decoding_key })
    }

    /// Loads this verifier from the secret enclosed in [`Config`].
    #[allow(dead_code)]
    pub fn from_config(cfg: &Config) -> Result<Self, JwtVerifierError> {
        Self::from_secret(&cfg.jwt_secret)
    }

    /// Decodes a given token received
    pub fn decode(&self, token: &str) -> Result<Claims, JwtVerifierError> {
        jsonwebtoken::decode::<Claims>(token, &self.decoding_key, &Validation::default())
            .map(|t| t.claims)
            .map_err(|e| {
                error!("{e:?}");
                JwtVerifierError::InvalidShape
            })
    }
}

impl CredentialVerifier for JwtVerifier {
    type Extension = Claims;

    type Error = JwtVerifierError;

    fn verify(
        &self,
        credential: &hyper::header::HeaderValue,
    ) -> Result<Option<Self::Extension>, Self::Error> {
        let raw: Vec<&str> = credential
            .to_str()
            .unwrap_or("")
            .split_whitespace()
            .collect();

        if raw.len() != 2 {
            error!("{}", JwtVerifierError::InvalidHeader);
            return Err(JwtVerifierError::InvalidHeader);
        }

        let token = raw[1];
        self.decode(token).map(Some)
    }

    fn unauthorized(&self, message: &str) -> Self::Error {
        JwtVerifierError::GenericMessage(tonic::Status::unauthenticated(message))
    }
}

/// A configuration only containing a JWT secret, just used for fast
/// on-the-fly construction with `from_env``
#[derive(Deserialize)]
#[allow(clippy::missing_docs_in_private_items)]
struct JwtConfig {
    jwt_secret: SecretString,
}

impl JwtConfig {
    /// Loads the secret from the environment
    fn from_env() -> Result<JwtConfig, envy::Error> {
        dotenvy::dotenv().ok();

        envy::prefixed("APP_").from_env::<JwtConfig>()
    }
}
