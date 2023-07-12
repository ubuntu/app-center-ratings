use super::entities::{User, UserId};
use super::{errors::RegisterError, infrastructure::create_user_in_db};

pub async fn create_user(instance_id: &str) -> Result<UserId, RegisterError> {
    tracing::info!("");

    if !validate_uid(instance_id) {
        return Err(RegisterError::InvalidUid);
    }

    let user = User::new(instance_id);

    create_user_in_db(user)
        .await
        .map(|user| user.id)
        .map_err(|err| {
            tracing::error!("{err:?}");

            RegisterError::FailedToCreateUserRecord
        })
}

pub const EXPECTED_UID_LENGTH: usize = 64;
pub const TOKEN_LENGTH: usize = 32;

fn validate_uid(uid: &str) -> bool {
    uid.len() == EXPECTED_UID_LENGTH
}
