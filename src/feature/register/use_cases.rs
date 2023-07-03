use super::{errors::RegisterError, infrastructure::create_user_in_db};
use crate::app::infrastructure::Infrastructure;
use rand::{distributions::Alphanumeric, Rng};

pub type Token = String;
pub type UserId = str;

#[tracing::instrument]
pub async fn create_user(uid: &UserId, infra: &Infrastructure) -> Result<Token, RegisterError> {
    tracing::info!("");

    if !validate_uid(&uid) {
        return Err(RegisterError::InvalidUid);
    }

    let token = create_token(&uid);
    create_user_in_db(token, infra).await
}

pub const EXPECTED_UID_LENGTH: usize = 64;
pub const TOKEN_LENGTH: usize = 32;

fn validate_uid(uid: &UserId) -> bool {
    uid.len() == EXPECTED_UID_LENGTH
}

fn create_token(_: &UserId) -> Token {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(TOKEN_LENGTH)
        .map(char::from)
        .collect()
}
