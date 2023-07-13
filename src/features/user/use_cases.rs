use crate::features::user::infrastructure::delete_user_by_user_id;

use super::entities::{User, UserId};
use super::{errors::UserError, infrastructure::create_user_in_db};

pub async fn create_user(user_id: &str) -> Result<UserId, UserError> {
    tracing::info!("");

    if !validate_user_id(user_id) {
        return Err(UserError::Invaliduser_id);
    }

    let user = User::new(user_id);

    create_user_in_db(user)
        .await
        .map(|user| user.id)
        .map_err(|err| {
            tracing::error!("{err:?}");

            UserError::FailedToCreateUserRecord
        })
}
pub async fn delete_user(user_id: &str) -> Result<(), UserError> {
    tracing::info!("");

    delete_user_by_user_id(user_id)
        .await
        .map(|rows_affected| ())
        .map_err(|err| {
            tracing::error!("{err:?}");
            UserError::FailedToDeleteUserRecord
        })
}

pub const EXPECTED_user_id_LENGTH: usize = 64;
pub const TOKEN_LENGTH: usize = 32;

fn validate_user_id(user_id: &str) -> bool {
    user_id.len() == EXPECTED_user_id_LENGTH
}
