//! Application level context & state
use crate::config::Config;
use jsonwebtoken::{EncodingKey, Header};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use time::{Duration, OffsetDateTime};
use tokio::sync::{Mutex, Notify};
use tracing::error;

/// How many days until JWT info expires
static JWT_EXPIRY_DAYS: i64 = 1;

/// Errors that can happen while encoding and signing tokens with JWT.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("jwt: error decoding secret: {0}")]
    DecodeSecretError(#[from] jsonwebtoken::errors::Error),

    #[error(transparent)]
    Envy(#[from] envy::Error),

    #[error("jwt: an error occurred, but the reason was erased for security reasons")]
    Erased,
}

pub struct Context {
    pub config: Config,
    pub jwt_encoder: JwtEncoder,
    pub http_client: reqwest::Client,
    /// In progress category updates that we need to block on
    pub category_updates: Mutex<HashMap<String, Arc<Notify>>>,
}

impl Context {
    pub fn new(config: &Config) -> Result<Self, Error> {
        Ok(Self {
            config: Config::load()?,
            jwt_encoder: JwtEncoder::from_secret(&config.jwt_secret)?,
            http_client: reqwest::Client::new(),
            category_updates: Default::default(),
        })
    }
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
        let exp = OffsetDateTime::now_utc() + Duration::days(JWT_EXPIRY_DAYS);
        let exp = exp.unix_timestamp() as usize;

        Self { sub, exp }
    }
}

pub struct JwtEncoder {
    encoding_key: EncodingKey,
}

impl JwtEncoder {
    pub fn from_secret(secret: &SecretString) -> Result<JwtEncoder, Error> {
        let encoding_key = EncodingKey::from_base64_secret(secret.expose_secret())?;

        Ok(Self { encoding_key })
    }

    pub fn encode(&self, sub: String) -> Result<String, Error> {
        let claims = Claims::new(sub);

        match jsonwebtoken::encode(&Header::default(), &claims, &self.encoding_key) {
            Ok(s) => Ok(s),
            Err(e) => {
                error!("unable to encode jwt: {e}");
                Err(Error::Erased)
            }
        }
    }
}
