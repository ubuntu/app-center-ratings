use crate::features::user::infrastructure::{find_user_votes, save_vote_to_db, user_seen};

use super::entities::{User, Vote};
use super::errors::UserError;
use super::infrastructure::{create_user_in_db, delete_user_by_client_hash};

pub async fn register(client_hash: &str) -> Result<User, UserError> {
    let user = User::new(client_hash);

    create_user_in_db(user).await.map_err(|err| {
        tracing::error!("{err:?}");
        UserError::FailedToCreateUserRecord
    })
}

pub async fn authenticate(id: &str) -> Result<bool, UserError> {
    user_seen(id).await.map_err(|error| {
        tracing::error!("{error:?}");
        UserError::InvalidUserId
    })
}

pub async fn delete_user(client_hash: &str) -> Result<(), UserError> {
    delete_user_by_client_hash(client_hash)
        .await
        .map(|_| ())
        .map_err(|error| {
            tracing::error!("{error:?}");
            UserError::FailedToDeleteUserRecord
        })
}

pub async fn vote(vote: Vote) -> Result<(), UserError> {
    save_vote_to_db(vote).await.map(|_| ()).map_err(|error| {
        tracing::error!("{error:?}");
        UserError::FailedToCastVote
    })
}

pub async fn list_my_votes(
    client_hash: String,
    snap_id_filter: Option<String>,
) -> Result<Vec<Vote>, UserError> {
    find_user_votes(client_hash, snap_id_filter)
        .await
        .map_err(|error| {
            tracing::error!("{error:?}");
            UserError::Unknown
        })
}
