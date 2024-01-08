use crate::{
    app::AppContext,
    features::user::infrastructure::{find_user_votes, save_vote_to_db},
    features::user::{
        entities::{User, Vote},
        errors::UserError,
        infrastructure::{
            create_or_seen_user, delete_user_by_client_hash, get_snap_votes_by_client_hash,
        },
    },
};

pub async fn authenticate(app_ctx: &AppContext, id: &str) -> Result<User, UserError> {
    let user = User::new(id);
    create_or_seen_user(app_ctx, user).await
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

pub async fn get_snap_votes(
    app_ctx: &AppContext,
    snap_id: String,
    client_hash: String,
) -> Result<Vec<Vote>, UserError> {
    get_snap_votes_by_client_hash(app_ctx, snap_id, client_hash).await
}

pub async fn list_my_votes(
    app_ctx: &AppContext,
    client_hash: String,
    snap_id_filter: Option<String>,
) -> Result<Vec<Vote>, UserError> {
    find_user_votes(app_ctx, client_hash, snap_id_filter).await
}
