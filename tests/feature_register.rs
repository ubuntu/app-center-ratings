mod clients;
mod utils;

use clients::register::{self, protobuf::CreateResponse};
use tonic::Code;

#[tokio::test]
async fn register_reject_empty_uid() {
    let uid = "";

    match register::do_create(uid).await {
        Ok(response) => panic!("expected Err but got Ok: {response:?}"),
        Err(status) => {
            assert_eq!(status.code(), Code::InvalidArgument)
        }
    }
}

#[tokio::test]
async fn register_accept_valid_uid() {
    let uid = "ea99b230006673cf88e45fa1af6d47f5269f939577adb1117ebaf7aa8aa0ec87";

    match register::do_create(uid).await {
        Ok(response) => {
            let CreateResponse { token } = response.into_inner();
            utils::assert::assert_token(&token)
        }
        Err(status) => {
            panic!("expected Ok but got Err: {status:?}")
        }
    }
}
