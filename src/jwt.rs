use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime};
use tonic::Status;
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

    #[error("jwt: invalid shape")]
    InvalidShape,

    #[error("jwt: invalid authz token")]
    InvalidHeader,

    #[error(transparent)]
    TonicStatus(#[from] Status),
}

impl From<Error> for Status {
    fn from(err: Error) -> Self {
        match err {
            Error::DecodeSecretError(_) => Status::unauthenticated("invalid JWT token"),
            Error::InvalidHeader => Status::unauthenticated("invalid authz header"),
            Error::TonicStatus(status) => status,
            _ => Status::internal("Internal Server Error"),
        }
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

#[derive(Clone)]
pub struct JwtVerifier {
    decoding_key: DecodingKey,
}

impl JwtVerifier {
    /// Creates a new verifier from the given secret.
    pub fn from_secret(secret: &SecretString) -> Result<Self, Error> {
        let decoding_key = DecodingKey::from_base64_secret(secret.expose_secret())?;

        Ok(Self { decoding_key })
    }

    pub fn decode(&self, token: &str) -> Result<Claims, Error> {
        jsonwebtoken::decode::<Claims>(token, &self.decoding_key, &Validation::default())
            .map(|t| t.claims)
            .map_err(|e| {
                error!("{e:?}");
                Error::InvalidShape
            })
    }
}
