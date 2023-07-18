use std::ops::Add;

use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use time::{Duration, OffsetDateTime};
use tracing::error;

use crate::utils::env;

static JWT_EXPIRY_IN_DAYS: i64 = 1;

#[derive(Error, Debug)]
pub enum JwtError {
    #[error("jwt: invalid shape")]
    InvalidShape,
    #[error("jwt: unknown error")]
    Unknown,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
}

impl Claims {
    pub fn new(sub: String) -> Self {
        let exp = OffsetDateTime::now_utc().add(Duration::days(JWT_EXPIRY_IN_DAYS));
        let exp = exp.unix_timestamp() as usize;

        Self { sub, exp }
    }
}

pub struct Jwt {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
}

impl Jwt {
    pub fn new() -> Self {
        let secret = env::get_config().jwt_secret.clone();
        let secret = secret.as_str();

        let encoding_key =
            EncodingKey::from_base64_secret(secret).expect("failed to load jwt secret");
        let decoding_key =
            DecodingKey::from_base64_secret(secret).expect("failed to load jwt secret");

        Self {
            encoding_key,
            decoding_key,
        }
    }

    pub fn encode(&self, sub: String) -> Result<String, JwtError> {
        let claims = Claims::new(sub);

        jsonwebtoken::encode(&Header::default(), &claims, &self.encoding_key).map_err(|e| {
            error!("{e:?}");
            JwtError::Unknown
        })
    }

    pub fn decode(&self, token: &str) -> Result<Claims, JwtError> {
        jsonwebtoken::decode::<Claims>(token, &self.decoding_key, &Validation::default())
            .map(|t| t.claims)
            .map_err(|e| {
                error!("{e:?}");
                JwtError::InvalidShape
            })
    }
}
