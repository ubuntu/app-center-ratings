use futures::FutureExt;
use ratings::app::AppContext;
use ratings::utils::{self, Infrastructure};
use sqlx::pool::PoolConnection;
use sqlx::{Postgres, Row};

use crate::helpers::client_user::pb::{AuthenticateResponse, RegisterResponse, VoteRequest};
use crate::helpers::client_user::UserClient;
use crate::helpers::with_lifecycle::with_lifecycle;

use utils::Config;

mod helpers;

#[derive(Debug)]
struct TestData {
    client: Option<UserClient>,
    id: Option<String>,
    token: Option<String>,
    app_ctx: AppContext,
}

impl TestData {
    async fn repository(&self) -> Result<PoolConnection<Postgres>, sqlx::Error> {
        self.app_ctx.clone().infrastructure().get_repository().await
    }

    fn socket(&self) -> String {
        self.app_ctx.config().get_socket()
    }
}

#[tokio::test]
async fn user_simple_lifecycle_test() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::load()?;
    let infra = Infrastructure::new(&config).await?;
    let app_ctx = AppContext::new(&config, infra);

    with_lifecycle(async {
        let data = TestData {
            client: Some(UserClient::new(&config.get_socket())),
            app_ctx,
            id: None,
            token: None,
        };
        register(data)
            .then(authenticate)
            .then(vote)
            .then(delete)
            .await;
    })
    .await;
    Ok(())
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
    helpers::assert::assert_token_is_valid(&token, &data.app_ctx.config().jwt_secret);

    let mut conn = data.repository().await.unwrap();

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
    helpers::assert::assert_token_is_valid(&token, &data.app_ctx.config().jwt_secret);

    // todo get last seen and compare

    data
}

async fn vote(data: TestData) -> TestData {
    let id = data.id.clone().unwrap();
    let token = data.token.clone().unwrap();
    let client = data.client.clone().unwrap();

    let expected_snap_id = "r4LxMVp7zWramXsJQAKdamxy6TAWlaDD";
    let expected_snap_revision = 111;
    let expected_vote_up = true;

    let ballet = VoteRequest {
        snap_id: expected_snap_id.to_string(),
        snap_revision: expected_snap_revision,
        vote_up: expected_vote_up,
    };

    client
        .vote(&token, ballet)
        .await
        .expect("vote should succeed")
        .into_inner();

    let mut conn = data.repository().await.unwrap();

    let result = sqlx::query(
        r#"
        SELECT votes.*
        FROM votes
        JOIN users ON votes.user_id_fk = users.id
        WHERE users.client_hash = $1 AND votes.snap_id = $2 AND votes.snap_revision = $3;
    "#,
    )
    .bind(&id)
    .bind(expected_snap_id)
    .bind(expected_snap_revision)
    .fetch_one(&mut *conn)
    .await
    .unwrap();

    let actual_snap_id: String = result.try_get("snap_id").unwrap();
    let actual_snap_revision: i32 = result.try_get("snap_revision").unwrap();
    let actual_vote_up: bool = result.try_get("vote_up").unwrap();

    assert_eq!(actual_snap_id, expected_snap_id);
    assert_eq!(actual_snap_revision, expected_snap_revision);
    assert_eq!(actual_vote_up, expected_vote_up);

    data
}

async fn delete(data: TestData) -> TestData {
    let token = data.token.clone().unwrap();
    let client = UserClient::new(&data.socket());
    client.delete(&token.clone()).await.unwrap();

    let id = data.id.clone().unwrap();

    let mut conn = data.repository().await.unwrap();

    let result = sqlx::query("SELECT * FROM users WHERE client_hash = $1")
        .bind(&id)
        .fetch_one(&mut *conn)
        .await;

    let Err(sqlx::Error::RowNotFound) = result else {
        panic!("The user {} still exists in the database or there was a database error", id);
    };

    data
}
