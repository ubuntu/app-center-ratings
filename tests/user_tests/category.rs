//! These tests require *specific* snaps because they do `snapd` lookups, so we can't
//! use the data-faked tests for this

use std::collections::HashSet;

use futures::{FutureExt, StreamExt};
use ratings::{
    app::AppContext,
    features::pb::{
        chart::Category,
        user::{GetSnapVotesRequest, VoteRequest},
    },
    utils::{Config, Infrastructure},
};
use sqlx::{pool::PoolConnection, Postgres, Row};

use super::simple_lifecycle_test::authenticate;
use crate::helpers::{
    client_user::UserClient, test_data::TestData, with_lifecycle::with_lifecycle,
};
use crate::{clear_test_snap, CLEAR_TEST_SNAP};

use super::super::{TESTING_SNAP_CATEGORIES, TESTING_SNAP_ID};

// Test getting the categories after casting a vote on our test snap, we can't use random
// data since this makes an actual `snapd` request so we use an unlisted test snap with some
// predictable categories set.
#[tokio::test]
async fn category_on_cast_vote() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::load()?;
    let infra = Infrastructure::new(&config).await?;
    let app_ctx = AppContext::new(&config, infra);

    CLEAR_TEST_SNAP.get_or_init(clear_test_snap).await;

    let data = TestData {
        user_client: Some(UserClient::new(&config.socket())),
        app_ctx,
        id: Some(TESTING_SNAP_ID.to_string()),
        token: None,
        app_client: None,
        snap_id: Some(TESTING_SNAP_ID.to_string()),
        chart_client: None,
        categories: Some(TESTING_SNAP_CATEGORIES.iter().cloned().collect()),
    };

    with_lifecycle(async {
        authenticate(data.clone()).then(vote).await;
    })
    .await;
    Ok(())
}

// Test getting the categories after getting the votes on our test snap, this isn't
// part of the earlier lifecycle because getting the votes is separate behavior from
// casting them and so it should be "clear" when we fetch the categories.
#[tokio::test]
async fn category_on_get_votes() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::load()?;
    let infra = Infrastructure::new(&config).await?;
    let app_ctx = AppContext::new(&config, infra);

    CLEAR_TEST_SNAP.get_or_init(clear_test_snap).await;

    let data = TestData {
        user_client: Some(UserClient::new(&config.socket())),
        app_ctx,
        id: None,
        token: None,
        app_client: None,
        snap_id: Some(TESTING_SNAP_ID.to_string()),
        chart_client: None,
        categories: Some(TESTING_SNAP_CATEGORIES.iter().cloned().collect()),
    };

    with_lifecycle(async {
        authenticate(data.clone()).then(vote).then(get_votes).await;
    })
    .await;
    Ok(())
}

async fn vote(data: TestData) -> TestData {
    let token = data.token.clone().unwrap();
    let client = data.user_client.clone().unwrap();

    let ballet = VoteRequest {
        snap_id: data.snap_id.clone().unwrap(),
        snap_revision: 2,
        vote_up: true,
    };

    client
        .vote(&token, ballet)
        .await
        .expect("vote should succeed")
        .into_inner();

    vote_sets_category(
        data.snap_id.as_ref().unwrap(),
        &mut data.repository().await.unwrap(),
        data.categories.as_ref().unwrap(),
    )
    .await;
    data
}

async fn get_votes(data: TestData) -> TestData {
    let token = data.token.clone().unwrap();
    let client = data.user_client.clone().unwrap();

    let request = GetSnapVotesRequest {
        snap_id: data.snap_id.clone().unwrap(),
    };
    client
        .get_snap_votes(&token, request)
        .await
        .expect("get votes should succeed");

    vote_sets_category(
        data.snap_id.as_ref().unwrap(),
        &mut data.repository().await.unwrap(),
        data.categories.as_ref().unwrap(),
    )
    .await;
    data
}

async fn vote_sets_category(
    snap_id: &str,
    conn: &mut PoolConnection<Postgres>,
    expected: &HashSet<Category>,
) {
    let expected: HashSet<_> = expected.iter().map(|v| v.to_kebab_case()).collect();
    let result = sqlx::query(
        r#"
        SELECT snap_categories.category
        FROM snap_categories
        WHERE snap_categories.snap_id = $1;
    "#,
    )
    .bind(snap_id)
    .fetch(&mut **conn)
    .map(|row| {
        row.expect("error when retrieving row")
            .try_get::<String, _>("category")
            .expect("could not get category field")
            .to_lowercase()
    })
    .collect::<HashSet<String>>()
    .await;

    assert_eq!(result, expected);
}
// 3Iwi803Tk3KQwyD6jFiAJdlq8MLgBIoD
