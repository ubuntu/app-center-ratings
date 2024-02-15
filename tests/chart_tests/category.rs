//! These tests require *specific* snaps because they do `snapd` lookups, so we can't
//! use the data-faked tests for this
//!
//! Warning! This actually causes problems if the number of votes is too big because the other
//! tests use the same DB and use *randomized* data, make sure you don't vote *too* much
//! on the test snap and break things.

use futures::FutureExt;
use ratings::{
    app::AppContext,
    features::pb::{
        chart::{Category, Timeframe},
        user::{AuthenticateResponse, VoteRequest},
    },
    utils::{Config, Infrastructure},
};

use crate::{
    clear_test_snap,
    helpers::{
        self, client_app::AppClient, client_chart::ChartClient, client_user::UserClient,
        test_data::TestData, vote_generator::generate_votes, with_lifecycle::with_lifecycle,
    },
    CLEAR_TEST_SNAP,
};

use super::super::{TESTING_SNAP_CATEGORIES, TESTING_SNAP_ID};

#[tokio::test]
async fn category_chart_filtering() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::load()?;
    let infra = Infrastructure::new(&config).await?;
    let app_ctx = AppContext::new(&config, infra);

    CLEAR_TEST_SNAP.get_or_init(clear_test_snap).await;

    let data = TestData {
        user_client: Some(UserClient::new(&config.socket())),
        app_ctx,
        id: None,
        token: None,
        app_client: Some(AppClient::new(&config.socket())),
        snap_id: Some(TESTING_SNAP_ID.to_string()),
        chart_client: Some(ChartClient::new(&config.socket())),
        categories: Some(TESTING_SNAP_CATEGORIES.iter().cloned().collect()),
    };

    with_lifecycle(async {
        vote(data)
            .then(multiple_votes)
            .then(is_in_right_category)
            .then(is_not_in_wrong_category)
            .await;
    })
    .await;
    Ok(())
}

/// Run a regular vote so that category information is gotten
async fn vote(mut data: TestData) -> TestData {
    let id: String = helpers::data_faker::rnd_sha_256();
    data.id = Some(id.to_string());

    let client = data.user_client.clone().unwrap();
    let response: AuthenticateResponse = client.authenticate(&id).await.unwrap().into_inner();
    let token: String = response.token;
    data.token = Some(token.to_string());

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
    data
}

// Does an app voted against multiple times appear correctly in the chart?
pub async fn multiple_votes(data: TestData) -> TestData {
    // This should rank our snap_id at the top of the chart, but only for our category
    generate_votes(&data.snap_id.clone().unwrap(), 111, true, 50, data.clone())
        .await
        .expect("Votes should succeed");

    data
}

async fn is_in_right_category(data: TestData) -> TestData {
    for category in data.categories.as_ref().unwrap() {
        let chart_data_result = data
            .chart_client
            .as_ref()
            .unwrap()
            .get_chart_of_category(
                Timeframe::Unspecified,
                Some(*category),
                &data.token.clone().unwrap(),
            )
            .await
            .expect("Get Chart should succeed")
            .into_inner()
            .ordered_chart_data;

        assert!(chart_data_result
            .into_iter()
            .filter_map(|v| v.rating.map(|v| v.snap_id))
            .any(|v| &v == data.snap_id.as_ref().unwrap()));
    }

    data
}

async fn is_not_in_wrong_category(data: TestData) -> TestData {
    debug_assert!(!data
        .categories
        .as_ref()
        .unwrap()
        .contains(&Category::ArtAndDesign));

    let chart_data_result = data
        .chart_client
        .as_ref()
        .unwrap()
        .get_chart_of_category(
            Timeframe::Unspecified,
            Some(Category::ArtAndDesign),
            &data.token.clone().unwrap(),
        )
        .await
        .expect("Get Chart should succeed")
        .into_inner()
        .ordered_chart_data;

    assert!(chart_data_result
        .into_iter()
        .filter_map(|v| v.rating.map(|v| v.snap_id))
        .all(|v| &v != data.snap_id.as_ref().unwrap()));

    data
}
