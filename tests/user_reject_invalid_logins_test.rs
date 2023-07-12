use tonic::Code;

use crate::helpers::client_user::UserClient;
use crate::helpers::with_lifecycle::with_lifecycle;

mod helpers;

#[tokio::test]
async fn blank() {
    with_lifecycle(async {
        let uid = "";
        let client = UserClient::default();

        match client.login(uid).await {
            Ok(response) => panic!("expected Err but got Ok: {response:?}"),
            Err(status) => {
                assert_eq!(status.code(), Code::InvalidArgument)
            }
        }
    })
    .await
}

#[tokio::test]
async fn wrong_length() {
    with_lifecycle(async {
        let uid = "foobarbazbun";
        let client = UserClient::default();

        match client.login(uid).await {
            Ok(response) => panic!("expected Err but got Ok: {response:?}"),
            Err(status) => {
                assert_eq!(status.code(), Code::InvalidArgument)
            }
        }
    })
    .await
}
