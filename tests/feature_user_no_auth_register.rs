mod utils;

use pb::RegisterResponse;
use tonic::{Code, Response, Status};
use utils::{
    lifecycle::{after, before},
    server_url,
};

use crate::pb::{user_no_auth_client::UserNoAuthClient, RegisterRequest};

pub mod pb {
    tonic::include_proto!("ratings.feature.user_no_auth");
}

async fn do_registration(uid: &str) -> Result<Response<RegisterResponse>, Status> {
    let url = server_url();
    let mut client = UserNoAuthClient::connect(url).await.unwrap();
    client
        .register(RegisterRequest {
            uid: uid.to_string(),
        })
        .await
}

#[tokio::test]
async fn feature_user_register_empty_uid() {
    before().await;

    let uid = "";

    match do_registration(uid).await {
        Ok(_) => todo!(),
        Err(status) => {
            assert_eq!(status.code(), Code::InvalidArgument)
        }
    }

    after().await;
}
