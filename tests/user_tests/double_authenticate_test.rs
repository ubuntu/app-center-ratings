use crate::helpers;
use crate::helpers::test_data::TestData;

use super::super::helpers::client_user::pb::AuthenticateResponse;
use super::super::helpers::client_user::UserClient;
use super::super::helpers::with_lifecycle::with_lifecycle;
use ratings::app::AppContext;
use ratings::utils::{self, Infrastructure};
use sqlx::Row;
use utils::Config;

#[tokio::test]
async fn authenticate_twice_test() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::load()?;
    let infra = Infrastructure::new(&config).await?;
    let app_ctx = AppContext::new(&config, infra);

    let data = TestData {
        user_client: Some(UserClient::new(&config.socket())),
        app_ctx,
        id: None,
        token: None,
        app_client: None,
        snap_id: None,
    };

    with_lifecycle(async {
        double_authenticate(data.clone()).await;
    })
    .await;
    Ok(())
}

async fn double_authenticate(data: TestData) {
    let id: String = helpers::data_faker::rnd_sha_256();
    let client = data.user_client.clone().unwrap();

    // First authenticate, registers user
    let response: AuthenticateResponse = client
        .authenticate(&id)
        .await
        .expect("authentication request should succeed")
        .into_inner();

    let first_token: String = response.token;
    helpers::assert::assert_token_is_valid(&first_token, &data.app_ctx.config().jwt_secret);

    let mut conn = data.repository().await.unwrap();
    let rows = sqlx::query("SELECT * FROM users WHERE client_hash = $1")
        .bind(&id)
        .fetch_one(&mut *conn)
        .await
        .unwrap();

    let actual: String = rows.get("client_hash");
    assert_eq!(actual, id);

    // Second authenticate
    let response: AuthenticateResponse = client
        .authenticate(&id)
        .await
        .expect("authentication request should succeed")
        .into_inner();

    let second_token: String = response.token;
    helpers::assert::assert_token_is_valid(&second_token, &data.app_ctx.config().jwt_secret);

    // User still registered
    let row = sqlx::query("SELECT COUNT(*) FROM users WHERE client_hash = $1")
        .bind(&id)
        .fetch_one(&mut *conn)
        .await
        .unwrap();

    let count: i64 = row.try_get("count").expect("Failed to get count");

    // Only appears in db once
    assert_eq!(count, 1);
}
