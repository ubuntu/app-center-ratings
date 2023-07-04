use tonic::Code;

use clients::user::{protobuf::LoginResponse, UserClient};

mod clients;
mod utils;

#[tokio::test]
async fn login_reject_empty_uid() {
    let uid = "";
    let client = UserClient::default();

    match client.login(uid).await {
        Ok(response) => panic!("expected Err but got Ok: {response:?}"),
        Err(status) => {
            assert_eq!(status.code(), Code::InvalidArgument)
        }
    }
}

#[tokio::test]
async fn login_accept_valid_uid() {
    utils::lifecycle::before().await;

    let uid = "ea99b230006673cf88e45fa1af6d47f5269f939577adb1117ebaf7aa8aa0ec87";
    let client = UserClient::default();

    match client.login(uid).await {
        Ok(response) => {
            let LoginResponse { token } = response.into_inner();
            utils::assert::assert_token_is_valid(&token)
        }
        Err(status) => {
            panic!("expected Ok but got Err: {status:?}")
        }
    }

    utils::lifecycle::after().await;
}
