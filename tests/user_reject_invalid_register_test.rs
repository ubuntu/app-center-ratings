use tonic::Code;

use crate::helpers::client_user::UserClient;
use crate::helpers::with_lifecycle::with_lifecycle;

mod helpers;

#[tokio::test]
async fn blank() {
    with_lifecycle(async {
        let id = "";
        let client = UserClient::new();

        match client.register(id).await {
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
        let client_hash = "foobarbazbun";
        let client = UserClient::new();

        match client.register(client_hash).await {
            Ok(response) => panic!("expected Err but got Ok: {response:?}"),
            Err(status) => {
                assert_eq!(status.code(), Code::InvalidArgument)
            }
        }
    })
    .await
}
