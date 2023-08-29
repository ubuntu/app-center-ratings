use crate::app::AppContext;
use crate::features::user::infrastructure::{find_user_votes, save_vote_to_db, user_seen};

use super::entities::{User, Vote};
use super::errors::UserError;
use super::infrastructure::{create_user_in_db, delete_user_by_client_hash};

pub async fn register(app_ctx: &AppContext, client_hash: &str) -> Result<User, UserError> {
    let user = User::new(client_hash);
    create_user_in_db(app_ctx, user).await
}

pub async fn authenticate(app_ctx: &AppContext, id: &str) -> Result<bool, UserError> {
    user_seen(app_ctx, id).await
}

pub async fn delete_user(app_ctx: &AppContext, client_hash: &str) -> Result<(), UserError> {
    let result = delete_user_by_client_hash(app_ctx, client_hash).await;
    result?;
    Ok(())
}

pub async fn vote(app_ctx: &AppContext, vote: Vote) -> Result<(), UserError> {
    let result = save_vote_to_db(app_ctx, vote).await;
    result?;
    Ok(())
}

pub async fn list_my_votes(
    app_ctx: &AppContext,
    client_hash: String,
    snap_id_filter: Option<String>,
) -> Result<Vec<Vote>, UserError> {
    find_user_votes(app_ctx, client_hash, snap_id_filter).await
}
