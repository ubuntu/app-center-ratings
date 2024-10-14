use std::sync::Arc;

use super::client::*;
use crate::helpers;
use futures::future::join_all;
use ratings::features::pb::user::{AuthenticateResponse, VoteRequest};
use thiserror::Error;
use tonic::Status;

#[derive(Clone, Debug, Error)]
pub enum GenerateVoteError {
    #[error("there was a panic while attempting to authenticate the votes: {0}")]
    Panic(String),
    #[error("there was a negative response from the server: {0}")]
    Status(#[from] Status),
}

impl From<String> for GenerateVoteError {
    fn from(value: String) -> Self {
        Self::Panic(value)
    }
}

pub async fn generate_votes(
    snap_id: &str,
    snap_revision: i32,
    vote_up: bool,
    count: u64,
    client: &TestClient,
) -> Result<(), GenerateVoteError> {
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
        join.map_err(|e| {
            if e.is_panic() {
                e.into_panic()
                    .downcast::<&'static str>()
                    .unwrap()
                    .to_string()
            } else {
                format!("other error: {}", e)
            }
        })??
    }

    Ok(())
}

async fn register_and_vote(
    snap_id: &str,
    snap_revision: i32,
    vote_up: bool,
    client: &TestClient,
) -> Result<(), Status> {
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
