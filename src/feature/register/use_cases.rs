use super::{errors::RegisterError, infrastructure::create_user_in_db};

pub type UserId = String;

pub async fn create_user(uid: &UserId) -> Result<UserId, RegisterError> {
    tracing::info!("");

    if !validate_uid(&uid) {
        return Err(RegisterError::InvalidUid);
    }

    create_user_in_db(uid).await
}

pub const EXPECTED_UID_LENGTH: usize = 64;
pub const TOKEN_LENGTH: usize = 32;

fn validate_uid(uid: &UserId) -> bool {
    uid.len() == EXPECTED_UID_LENGTH
}
