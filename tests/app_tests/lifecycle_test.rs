use futures::FutureExt;
use ratings::{
    app::AppContext,
    utils::{Config, Infrastructure, Migrator},
};

use super::super::helpers::with_lifecycle::with_lifecycle;
use crate::helpers::test_data::TestData;
use crate::helpers::vote_generator::generate_votes;
use crate::helpers::{self, client_app::pb::RatingsBand, client_app::AppClient};
use crate::helpers::{
    client_user::{pb::RegisterResponse, UserClient},
    data_faker,
};

#[tokio::test]
async fn app_lifecycle_test() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::load()?;
    let infra = Infrastructure::new(&config).await?;
    let app_ctx = AppContext::new(&config, infra);

    let migrator = Migrator::new(&config.migration_postgres_uri).await?;
    let data = TestData {
        user_client: Some(UserClient::new(&config.socket())),
        app_ctx,
        id: None,
        token: None,
        app_client: Some(AppClient::new(&config.socket())),
        snap_id: Some(data_faker::rnd_id()),
    };

    with_lifecycle(
        async {
            vote_once(data.clone()).then(vote_up).await;
        },
        migrator,
    )
    .await;
    Ok(())
}

async fn vote_once(mut data: TestData) -> TestData {
    let vote_up = true;
    let expected_total_votes = 1;
    let expected_rating_band = RatingsBand::InsufficientVotes;

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
    let response: RegisterResponse = client.register(&id).await.unwrap().into_inner();
    let token: String = response.token;
    data.token = Some(token.to_string());

    let result = data
        .clone()
        .app_client
        .unwrap()
        .get_rating(&data.snap_id.clone().unwrap(), &data.token.clone().unwrap())
        .await
        .expect("Get Rating should succeed")
        .into_inner()
        .rating
        .unwrap();

    let actual_snap_id = result.snap_id;
    let actual_total_votes = result.total_votes;
    let actual_ratings_band = result.ratings_band;

    assert_eq!(data.snap_id.clone().unwrap(), actual_snap_id);
    assert_eq!(expected_total_votes, actual_total_votes);
    assert_eq!(expected_rating_band as i32, actual_ratings_band);

    data
}
async fn vote_up(data: TestData) -> TestData {
    let vote_up = true;
    let expected_total_votes = 26;
    let expected_rating_band = RatingsBand::VeryGood;

    generate_votes(
        &data.snap_id.clone().unwrap(),
        111,
        vote_up,
        25,
        data.clone(),
    )
    .await
    .expect("Votes should succeed");

    let result = data
        .clone()
        .app_client
        .unwrap()
        .get_rating(&data.snap_id.clone().unwrap(), &data.token.clone().unwrap())
        .await
        .expect("Get Rating should succeed")
        .into_inner()
        .rating
        .unwrap();

    let actual_snap_id = result.snap_id;
    let actual_total_votes = result.total_votes;
    let actual_ratings_band = result.ratings_band;

    assert_eq!(data.snap_id.clone().unwrap(), actual_snap_id);
    assert_eq!(expected_total_votes, actual_total_votes);
    assert_eq!(expected_rating_band as i32, actual_ratings_band);

    data
}
