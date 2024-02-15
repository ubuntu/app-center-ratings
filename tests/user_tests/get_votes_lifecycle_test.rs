use crate::helpers;
use crate::helpers::test_data::TestData;

use super::super::helpers::client_user::UserClient;
use super::super::helpers::with_lifecycle::with_lifecycle;
use futures::FutureExt;
use ratings::app::AppContext;
use ratings::features::pb::user::{AuthenticateResponse, GetSnapVotesRequest, VoteRequest};
use ratings::utils::{self, Infrastructure};
use sqlx::Row;

use utils::Config;

#[tokio::test]
async fn get_votes_lifecycle_test() -> Result<(), Box<dyn std::error::Error>> {
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
        chart_client: None,
        categories: None,
    };

    with_lifecycle(async {
        authenticate(data.clone())
            .then(cast_vote)
            .then(get_votes)
            .await;
    })
    .await;
    Ok(())
}

async fn authenticate(mut data: TestData) -> TestData {
    let id: String = helpers::data_faker::rnd_sha_256();
    data.id = Some(id.to_string());

    let client = data.user_client.clone().unwrap();
    let response: AuthenticateResponse = client
        .authenticate(&id)
        .await
        .expect("authentication request should succeed")
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

async fn cast_vote(data: TestData) -> TestData {
    let id = data.id.clone().unwrap();
    let token = data.token.clone().unwrap();
    let client = data.user_client.clone().unwrap();

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

    let expected_snap_id = "r4LxMVp7zWramXsJQAKdamxy6TAWlaDD";
    let expected_snap_revision = 112;
    let expected_vote_up = false;

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

async fn get_votes(data: TestData) -> TestData {
    let token = data.token.clone().unwrap();
    let client = data.user_client.clone().unwrap();

    let expected_snap_id = "r4LxMVp7zWramXsJQAKdamxy6TAWlaDD".to_string();
    let expected_first_revision = 111;
    let expected_first_vote_up = true;
    let expected_second_revision = 112;
    let expected_second_vote_up = false;

    let request = GetSnapVotesRequest {
        snap_id: expected_snap_id.clone(),
    };
    let votes = client
        .get_snap_votes(&token, request)
        .await
        .expect("get votes should succeed")
        .into_inner()
        .votes;

    let actual_snap_id = &votes[0].snap_id;
    let actual_first_revision = votes[0].snap_revision;
    let actual_first_vote_up = votes[0].vote_up;
    let actual_second_revision = votes[1].snap_revision;
    let actual_second_vote_up = votes[1].vote_up;

    assert_eq!(actual_snap_id, &expected_snap_id);
    assert_eq!(actual_first_revision, expected_first_revision);
    assert_eq!(actual_first_vote_up, expected_first_vote_up);
    assert_eq!(actual_second_vote_up, expected_second_vote_up);
    assert_eq!(actual_second_revision, expected_second_revision);
    data
}
