use super::super::helpers::client_user::pb::{AuthenticateResponse, VoteRequest};
use super::test_data::TestData;
use crate::helpers;

pub async fn generate_votes(
    snap_id: &str,
    snap_revision: i32,
    vote_up: bool,
    count: u64,
    data: TestData,
) -> Result<(), Box<dyn std::error::Error>> {
    for _ in 0..count {
        register_and_vote(snap_id, snap_revision, vote_up, data.clone()).await?;
    }
    Ok(())
}

async fn register_and_vote(
    snap_id: &str,
    snap_revision: i32,
    vote_up: bool,
    data: TestData,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = data.user_client.clone().unwrap();
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
