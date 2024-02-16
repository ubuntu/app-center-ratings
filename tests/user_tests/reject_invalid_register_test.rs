use ratings::utils::Config;
use tonic::Code;

use super::super::helpers::{client_user::*, with_lifecycle::with_lifecycle};

#[tokio::test]
async fn blank() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::load()?;

    with_lifecycle(async {
        let id = "";
        let client = UserClient::new(config.socket());

        match client.authenticate(id).await {
            Ok(response) => panic!("expected Err but got Ok: {response:?}"),
            Err(status) => {
                assert_eq!(status.code(), Code::InvalidArgument)
            }
        }
    })
    .await;
    Ok(())
}

#[tokio::test]
async fn wrong_length() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::load()?;

    with_lifecycle(async {
        let client_hash = "foobarbazbun";
        let client = UserClient::new(config.socket());

        match client.authenticate(client_hash).await {
            Ok(response) => panic!("expected Err but got Ok: {response:?}"),
            Err(status) => {
                assert_eq!(status.code(), Code::InvalidArgument)
            }
        }
    })
    .await;
    Ok(())
}
