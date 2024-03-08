//! Definitions meant to help with JWT handling throughout the app.

use jsonwebtoken::{EncodingKey, Header};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use time::{Duration, OffsetDateTime};
use tracing::error;

use super::Config;

/// How many days until JWT info expires
static JWT_EXPIRY_IN_DAYS: i64 = 1;

/// Information representating a claim on a specific subject at a specific time
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    /// The subject
    pub sub: String,
    /// The expiration time
    pub exp: usize,
}

impl Claims {
    /// Creates a new claim with the current datetime for the subject given by `sub`.
    pub fn new(sub: String) -> Self {
        let exp = OffsetDateTime::now_utc() + Duration::days(JWT_EXPIRY_IN_DAYS);
        let exp = exp.unix_timestamp() as usize;

        Self { sub, exp }
    }
}

/// Errors that can happen while encoding and signing tokens with JWT.
#[derive(Error, Debug)]
#[allow(missing_docs)]
pub enum JwtEncoderError {
    #[error("jwt: error decoding secret: {0}")]
    DecodeSecretError(#[from] jsonwebtoken::errors::Error),
    #[error("jwt: an error occurred, but the reason was erased for security reasons")]
    Erased,
}

/// An encoder which allows converting user hashes into valid JWT tokens
pub struct JwtEncoder {
    /// An encoding key for transfer
    encoding_key: EncodingKey,
}

impl JwtEncoder {
    /// Loads the encoder from the given JWT secret
    pub fn from_secret(secret: &SecretString) -> Result<JwtEncoder, JwtEncoderError> {
        let encoding_key = EncodingKey::from_base64_secret(secret.expose_secret())?;
        Ok(Self { encoding_key })
    }

    /// Loads our encoder from the secret enclosed in [`Config`]
    pub fn from_config(config: &Config) -> Result<JwtEncoder, JwtEncoderError> {
        Self::from_secret(&config.jwt_secret)
    }

    /// Encodes a token for use
    pub fn encode(&self, sub: String) -> Result<String, JwtEncoderError> {
        let claims = Claims::new(sub);

        jsonwebtoken::encode(&Header::default(), &claims, &self.encoding_key).map_err(|e| {
            error!("{e}");
            JwtEncoderError::Erased
        })
    }
}
