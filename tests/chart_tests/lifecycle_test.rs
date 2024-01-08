use futures::FutureExt;
use ratings::{
    app::AppContext,
    utils::{Config, Infrastructure},
};

use super::super::helpers::with_lifecycle::with_lifecycle;
use crate::helpers::vote_generator::generate_votes;
use crate::helpers::{self, client_app::AppClient};
use crate::helpers::{client_user::UserClient, data_faker};
use crate::pb::common::RatingsBand;
use crate::pb::user::AuthenticateResponse;
use crate::{
    helpers::{client_chart::ChartClient, test_data::TestData},
    pb::{chart::Timeframe, common::Rating},
};

#[tokio::test]
async fn chart_lifecycle_test() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::load()?;
    let infra = Infrastructure::new(&config).await?;
    let app_ctx = AppContext::new(&config, infra);

    let data = TestData {
        user_client: Some(UserClient::new(&config.socket())),
        app_ctx,
        id: None,
        token: None,
        app_client: Some(AppClient::new(&config.socket())),
        snap_id: Some(data_faker::rnd_id()),
        chart_client: Some(ChartClient::new(&config.socket())),
    };

    with_lifecycle(async {
        vote_once(data.clone())
            .then(multiple_votes)
            .then(timeframed_votes_dont_appear)
            .await;
    })
    .await;
    Ok(())
}

// Does an app voted against once appear correctly in the chart?
async fn vote_once(mut data: TestData) -> TestData {
    let vote_up = true;

    // Fill up chart with other votes so ours doesn't appear
    for _ in 0..20 {
        generate_votes(&data_faker::rnd_id(), 111, vote_up, 25, data.clone())
            .await
            .expect("Votes should succeed");
    }

    let vote_up = true;

    generate_votes(
        &data.snap_id.clone().unwrap(),
        111,
        vote_up,
        1,
        data.clone(),
    )
    .await
    .expect("Votes should succeed");

    let id: String = helpers::data_faker::rnd_sha_256();
    data.id = Some(id.to_string());

    let client = data.user_client.clone().unwrap();
    let response: AuthenticateResponse = client.authenticate(&id).await.unwrap().into_inner();
    let token: String = response.token;
    data.token = Some(token.to_string());

    let timeframe = Timeframe::Unspecified;

    let chart_data_result = data
        .clone()
        .chart_client
        .unwrap()
        .get_chart(timeframe, &data.token.clone().unwrap())
        .await
        .expect("Get Chart should succeed")
        .into_inner()
        .ordered_chart_data;

    let result = chart_data_result.into_iter().find(|chart_data| {
        if let Some(rating) = &chart_data.rating {
            rating.snap_id == data.snap_id.clone().unwrap()
        } else {
            false
        }
    });

    // Should not appear in chart
    assert_eq!(result, None);

    data
}

// Does an app voted against multiple times appear correctly in the chart?
async fn multiple_votes(mut data: TestData) -> TestData {
    let vote_up = true;
    let expected_raw_rating = 0.8;
    let expected_rating = Rating {
        snap_id: data.snap_id.clone().unwrap(),
        total_votes: 101,
        ratings_band: RatingsBand::VeryGood.into(),
    };

    // This should rank our snap_id at the top of the chart
    generate_votes(
        &data.snap_id.clone().unwrap(),
        111,
        vote_up,
        100,
        data.clone(),
    )
    .await
    .expect("Votes should succeed");

    let id: String = helpers::data_faker::rnd_sha_256();
    data.id = Some(id.to_string());

    let client = data.user_client.clone().unwrap();
    let response: AuthenticateResponse = client.authenticate(&id).await.unwrap().into_inner();
    let token: String = response.token;
    data.token = Some(token.to_string());

    let timeframe = Timeframe::Unspecified;

    let chart_data_result = data
        .clone()
        .chart_client
        .unwrap()
        .get_chart(timeframe, &data.token.clone().unwrap())
        .await
        .expect("Get Chart should succeed")
        .into_inner()
        .ordered_chart_data;

    // Should be at the top of the chart
    if let Some(chart_data) = chart_data_result.first() {
        let actual_rating = chart_data.rating.clone().expect("Rating should exist");
        let actual_raw_rating = chart_data.raw_rating;

        assert_eq!(expected_rating, actual_rating);
        assert!(expected_raw_rating < actual_raw_rating);
    } else {
        panic!("No chart data available");
    }

    data
}

// Does the Timeframe correctly filter out app data?
async fn timeframed_votes_dont_appear(mut data: TestData) -> TestData {
    let mut conn = data.repository().await.unwrap();

    // Timewarp the votes back two months so they are out of the requested timeframe
    sqlx::query("UPDATE votes SET created = created - INTERVAL '2 months' WHERE snap_id = $1")
        .bind(&data.snap_id.clone().unwrap())
        .execute(&mut *conn)
        .await
        .unwrap();

    let id: String = helpers::data_faker::rnd_sha_256();
    data.id = Some(id.to_string());

    let client = data.user_client.clone().unwrap();
    let response: AuthenticateResponse = client.authenticate(&id).await.unwrap().into_inner();
    let token: String = response.token;
    data.token = Some(token.to_string());

    let timeframe = Timeframe::Month;

    let chart_data_result = data
        .clone()
        .chart_client
        .unwrap()
        .get_chart(timeframe, &data.token.clone().unwrap())
        .await
        .expect("Get Chart should succeed")
        .into_inner()
        .ordered_chart_data;

    let result = chart_data_result.into_iter().find(|chart_data| {
        if let Some(rating) = &chart_data.rating {
            rating.snap_id == data.snap_id.clone().unwrap()
        } else {
            false
        }
    });

    // Should no longer find the ratings as they are too old
    assert_eq!(result, None);

    let expected_raw_rating = 0.8;
    let expected_rating = Rating {
        snap_id: data.snap_id.clone().unwrap(),
        total_votes: 101,
        ratings_band: RatingsBand::VeryGood.into(),
    };

    // Unspecified timeframe should now pick up the ratings again
    let timeframe = Timeframe::Unspecified;
    let chart_data_result = data
        .clone()
        .chart_client
        .unwrap()
        .get_chart(timeframe, &data.token.clone().unwrap())
        .await
        .expect("Get Chart should succeed")
        .into_inner()
        .ordered_chart_data;

    let result = chart_data_result.into_iter().find(|chart_data| {
        if let Some(rating) = &chart_data.rating {
            rating.snap_id == data.snap_id.clone().unwrap()
        } else {
            false
        }
    });

    let actual_rating = result.clone().unwrap().rating.unwrap();
    let actual_raw_rating = result.unwrap().raw_rating;

    assert_eq!(expected_rating, actual_rating);
    assert!(expected_raw_rating < actual_raw_rating);

    data
}
