use super::client_user::*;
use crate::helpers;
use ratings::features::pb::user::{AuthenticateResponse, VoteRequest};

pub async fn generate_votes(
    snap_id: &str,
    snap_revision: i32,
    vote_up: bool,
    count: u64,
    client: &UserClient,
) -> Result<(), Box<dyn std::error::Error>> {
    for _ in 0..count {
        register_and_vote(snap_id, snap_revision, vote_up, client).await?;
    }
    Ok(())
}

async fn register_and_vote(
    snap_id: &str,
    snap_revision: i32,
    vote_up: bool,
    client: &UserClient,
) -> Result<(), Box<dyn std::error::Error>> {
    let id: String = helpers::data_faker::rnd_sha_256();
    let response: AuthenticateResponse = client
        .authenticate(&id)
        .await
        .expect("register request should succeed")
        .into_inner();
    let token: String = response.token;

    let _: AuthenticateResponse = client
        .authenticate(&id)
        .await
        .expect("authenticate should succeed")
        .into_inner();

    let ballet = VoteRequest {
        snap_id: snap_id.to_string(),
        snap_revision,
        vote_up,
    };

    client
        .vote(&token, ballet)
        .await
        .expect("vote should succeed")
        .into_inner();
    Ok(())
}
