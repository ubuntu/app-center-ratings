use sqlx::Row;
use tonic::Code;

use clients::user::{protobuf::LoginResponse, UserClient};

use crate::utils::infra::get_repository;

mod clients;
mod repositories;
mod utils;

struct TestData {
    foo: String,
}

#[tokio::test]
async fn user_integration_tests() {
    let mut shared = TestData {
        foo: "hi".to_string(),
    };

    test1(&mut shared).await;
    test2(&mut shared).await;
    test3(&mut shared).await;

    // success!
}

async fn test1(ctx: &mut TestData) {
    ctx.foo.push_str("11")
}

async fn test2(ctx: &mut TestData) {
    ctx.foo.push_str("22")
}

async fn test3(ctx: &mut TestData) {
    ctx.foo.push_str("33")
}

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

    let instance_id = "ea99b230006673cf88e45fa1af6d47f5269f939577adb1117ebaf7aa8aa0ec88";
    let client = UserClient::default();
    let response = client
        .login(instance_id)
        .await
        .expect("request should have succeeded");
    let LoginResponse { token } = response.into_inner();

    // check jwt is valid
    utils::assert::assert_token_is_valid(&token);

    let mut conn = get_repository().await;
    let rows = sqlx::query("SELECT * FROM users WHERE instance_id = $1")
        .bind(&instance_id)
        .fetch_one(&mut *conn)
        .await
        .unwrap();

    let actual: String = rows.get("instance_id");

    assert_eq!(actual, instance_id);

    utils::lifecycle::after().await;
}
