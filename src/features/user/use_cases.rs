use crate::features::user::infrastructure::delete_user_by_instance_id;

use super::entities::{User, UserId};
use super::{errors::UserError, infrastructure::create_user_in_db};

pub async fn create_user(instance_id: &str) -> Result<UserId, UserError> {
    tracing::info!("");

    if !validate_uid(instance_id) {
        return Err(UserError::InvalidUid);
    }

    let user = User::new(instance_id);

    create_user_in_db(user)
        .await
        .map(|user| user.id)
        .map_err(|err| {
            tracing::error!("{err:?}");

            UserError::FailedToCreateUserRecord
        })
}
pub async fn delete_user(instance_id: &str) -> Result<(), UserError> {
    tracing::info!("");

    delete_user_by_instance_id(instance_id)
        .await
        .map(|rows_affected| ())
        .map_err(|err| {
            tracing::error!("{err:?}");
            UserError::FailedToDeleteUserRecord
        })
}

pub const EXPECTED_UID_LENGTH: usize = 64;
pub const TOKEN_LENGTH: usize = 32;

fn validate_uid(uid: &str) -> bool {
    uid.len() == EXPECTED_UID_LENGTH
}
