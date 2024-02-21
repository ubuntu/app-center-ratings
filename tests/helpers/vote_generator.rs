use std::sync::Arc;

use super::client_user::*;
use crate::helpers;
use futures::future::join_all;
use ratings::features::pb::user::{AuthenticateResponse, VoteRequest};

pub async fn generate_votes(
    snap_id: &str,
    snap_revision: i32,
    vote_up: bool,
    count: u64,
    client: &UserClient,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut joins = Vec::with_capacity(count as usize);

    let snap_id = Arc::new(snap_id.to_string());
    let client = Arc::new(client.clone());
    for _ in 0..count {
        let snap_id = snap_id.clone();
        let client = client.clone();
        joins.push(tokio::spawn(async move {
            register_and_vote(&snap_id, snap_revision, vote_up, &client).await
        }));
    }

    for join in join_all(joins).await {
        // The second ? returns an error about type sizing for some reason
        #[allow(clippy::question_mark)]
        if let Err(err) = join? {
            return Err(err);
        }
    }

    Ok(())
}

async fn register_and_vote(
    snap_id: &str,
    snap_revision: i32,
    vote_up: bool,
    client: &UserClient,
) -> Result<(), Box<dyn std::error::Error + Send>> {
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
