use futures::FutureExt;
use sqlx::Row;

use crate::helpers::client_user::{AuthenticateResponse, RegisterResponse, UserClient};
use crate::helpers::infrastructure::get_repository;
use crate::helpers::with_lifecycle::with_lifecycle;

mod helpers;

#[derive(Debug, Default)]
struct TestData {
    client: Option<UserClient>,
    id: Option<String>,
    token: Option<String>,
}

#[tokio::test]
async fn user_simple_lifecycle_test() {
    with_lifecycle(async {
        let mut data = TestData::default();
        data.client = Some(UserClient::new());

        register(data).then(authenticate).then(delete).await;
    })
    .await;
}

async fn register(mut data: TestData) -> TestData {
    let id: String = helpers::data_faker::rnd_sha_256();
    data.id = Some(id.to_string());

    let client = data.client.clone().unwrap();
    let response: RegisterResponse = client
        .register(&id)
        .await
        .expect("register request should succeed")
        .into_inner();

    let token: String = response.token;
    data.token = Some(token.to_string());
    helpers::assert::assert_token_is_valid(&token);

    let mut conn = get_repository().await;
    let rows = sqlx::query("SELECT * FROM users WHERE client_hash = $1")
        .bind(&id)
        .fetch_one(&mut *conn)
        .await
        .unwrap();

    let actual: String = rows.get("client_hash");

    assert_eq!(actual, id);

    data
}

async fn authenticate(mut data: TestData) -> TestData {
    let id = data.id.clone().unwrap();
    let client = data.client.clone().unwrap();

    // todo get last seen

    let response: AuthenticateResponse = client
        .authenticate(&id)
        .await
        .expect("authenticate should succeed")
        .into_inner();

    let token: String = response.token;
    data.token = Some(token.to_string());
    helpers::assert::assert_token_is_valid(&token);

    // todo get last seen and compare

    data
}

async fn delete(data: TestData) -> TestData {
    let token = data.token.clone().unwrap();
    let client = UserClient::new();
    client.delete(&token.clone()).await.unwrap();

    let id = data.id.clone().unwrap();
    let mut conn = get_repository().await;
    let result = sqlx::query("SELECT * FROM users WHERE client_hash = $1")
        .bind(&id)
        .fetch_one(&mut *conn)
        .await;

    let Err(sqlx::Error::RowNotFound) = result else {
        panic!("The user {} still exists in the database or there was a database error", id);
    };

    data
}
