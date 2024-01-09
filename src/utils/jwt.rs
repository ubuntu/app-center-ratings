//! JSON Web Tokens infrastructure and utlities.
use std::ops::Add;

use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use time::{Duration, OffsetDateTime};
use tracing::error;

/// How many days until JWT info expires
static JWT_EXPIRY_IN_DAYS: i64 = 1;

/// An error for things that can go wrong with JWT handling
#[derive(Error, Debug)]
pub enum JwtError {
    /// The shape of the data is invalid
    #[error("jwt: invalid shape")]
    InvalidShape,
    /// Anything else that can go wrong
    #[error("jwt: unknown error")]
    Unknown,
}

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
        let exp = OffsetDateTime::now_utc().add(Duration::days(JWT_EXPIRY_IN_DAYS));
        let exp = exp.unix_timestamp() as usize;

        Self { sub, exp }
    }
}

/// A JWT transaction representation
pub struct Jwt {
    /// An encoding key for transfer
    encoding_key: EncodingKey,
    /// A decoding key for receipt
    decoding_key: DecodingKey,
}

impl Jwt {
    /// Attempts to create a new JWT representation from a given secret
    pub fn new(secret: &str) -> Result<Self, jsonwebtoken::errors::Error> {
        let encoding_key = EncodingKey::from_base64_secret(secret)?;
        let decoding_key = DecodingKey::from_base64_secret(secret)?;

        Ok(Self {
            encoding_key,
            decoding_key,
        })
    }

    /// Encodes a token for use
    pub fn encode(&self, sub: String) -> Result<String, JwtError> {
        let claims = Claims::new(sub);

        jsonwebtoken::encode(&Header::default(), &claims, &self.encoding_key).map_err(|e| {
            error!("{e:?}");
            JwtError::Unknown
        })
    }

    /// Decodes a given token received
    pub fn decode(&self, token: &str) -> Result<Claims, JwtError> {
        jsonwebtoken::decode::<Claims>(token, &self.decoding_key, &Validation::default())
            .map(|t| t.claims)
            .map_err(|e| {
                error!("{e:?}");
                JwtError::InvalidShape
            })
    }
}
