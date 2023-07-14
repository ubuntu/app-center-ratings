use crate::features::user::infrastructure::{save_vote_to_db, user_seen};

use super::entities::{User, Vote};
use super::errors::UserError;
use super::infrastructure::{create_user_in_db, delete_user_by_user_id};

pub async fn register(user_id: &str) -> Result<User, UserError> {
    let user = User::new(user_id);

    create_user_in_db(user).await.map_err(|err| {
        tracing::error!("{err:?}");
        UserError::FailedToCreateUserRecord
    })
}

pub async fn authenticate(user_id: &str) -> Result<bool, UserError> {
    user_seen(user_id).await.map_err(|error| {
        tracing::error!("{error:?}");
        UserError::InvalidUserId
    })
}

pub async fn delete_user(user_id: &str) -> Result<(), UserError> {
    delete_user_by_user_id(user_id)
        .await
        .map(|rows_affected| ())
        .map_err(|error| {
            tracing::error!("{error:?}");
            UserError::FailedToDeleteUserRecord
        })
}

pub async fn vote(vote: Vote) -> Result<(), UserError> {
    save_vote_to_db(vote)
        .await
        .map(|rows_affected| ())
        .map_err(|error| {
            tracing::error!("{error:?}");
            UserError::FailedToDeleteUserRecord
        })
}
