#![cfg(test)]

use cucumber::{given, then, when, Parameter, World};
use futures::FutureExt;
use helpers::client::*;
use rand::{thread_rng, Rng};
use ratings::{
    app::AppContext, features::{
        common::entities::{calculate_band, VoteSummary},
        pb::chart::{Category, ChartData, Timeframe},
    }, utils::{Config, Infrastructure}
};
use sqlx::Connection;
use strum::EnumString;

mod helpers;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Parameter, EnumString)]
#[param(name = "category", regex = "Utilities|Development")]
pub enum TestCategory {
    Utilities,
    Development,
}

impl From<TestCategory> for Category {
    fn from(value: TestCategory) -> Self {
        match value {
            TestCategory::Development => Self::Development,
            TestCategory::Utilities => Self::Utilities,
        }
    }
}

#[derive(Debug, World)]
#[world(init = Self::new)]
struct ChartWorld {
    token: String,
    snap_ids: Vec<String>,
    test_snap: String,
    client: TestClient,
    chart_data: Vec<ChartData>,
}

impl ChartWorld {
    async fn new() -> Self {
        let config = Config::load().expect("could not load config");
        let client = TestClient::new(config.socket());

        let token = client
            .authenticate(&helpers::data_faker::rnd_sha_256())
            .await
            .expect("could not authenticate test client")
            .into_inner()
            .token;

        Self {
            snap_ids: Vec::with_capacity(25),
            test_snap: Default::default(),
            chart_data: Vec::new(),
            client,
            token,
        }
    }
}

#[given(expr = "a snap with id {string} gets {int} votes where {int} are upvotes")]
async fn set_test_snap(world: &mut ChartWorld, snap_id: String, votes: usize, upvotes: usize) {
    world.test_snap = snap_id;

    helpers::vote_generator::generate_votes(
        &world.test_snap,
        1,
        true,
        upvotes as u64,
        &world.client,
    )
    .await
    .expect("could not generate votes");

    tracing::debug!("done generating upvotes");

    helpers::vote_generator::generate_votes(
        &world.test_snap,
        1,
        false,
        (votes - upvotes) as u64,
        &world.client,
    )
    .await
    .expect("could not generate votes");

    tracing::debug!("done generating downvotes");
}

#[given(
    expr = "{int} test snaps gets between {int} and {int} votes, where {int} to {int} are upvotes"
)]
async fn generate_snaps(
    world: &mut ChartWorld,
    num_snaps: usize,
    min_vote: usize,
    max_vote: usize,
    min_upvote: usize,
    max_upvote: usize,
) {
    let mut expected = Vec::with_capacity(num_snaps);

    for i in 1..=num_snaps {
        tracing::debug!("starting snap {i} / {num_snaps}");

        let (upvotes, votes) = {
            let mut rng = thread_rng();

            let upvotes = rng.gen_range(min_upvote..max_upvote);
            let min_vote = Ord::max(upvotes, min_vote);
            let votes = rng.gen_range(min_vote..=max_vote);
            (upvotes, votes)
        };

        let id = helpers::data_faker::rnd_id();

        helpers::vote_generator::generate_votes(&id, 1, true, upvotes as u64, &world.client)
            .await
            .expect("could not generate votes");

        tracing::debug!("done generating upvotes ({i} / {num_snaps})");

        helpers::vote_generator::generate_votes(
            &id,
            1,
            false,
            (votes - upvotes) as u64,
            &world.client,
        )
        .await
        .expect("could not generate votes");

        tracing::debug!("done generating downvotes ({i} / {num_snaps})");

        let summary = VoteSummary {
            snap_id: id,
            total_votes: votes as i64,
            positive_votes: upvotes as i64,
        };

        expected.push((calculate_band(&summary).0.unwrap(), summary.snap_id));
    }

    expected.sort_unstable_by(|(band1, _), (band2, _)| band1.partial_cmp(band2).unwrap().reverse());
    world.snap_ids.extend(expected.drain(..).map(|(band, id)| {
        tracing::debug!("id: {id}; band: {band}");
        id
    }));
}

#[given(expr = "the database is warmed up")]
async fn warmup(_: &mut ChartWorld) {
    let config = Config::load().unwrap();
    let infra = Infrastructure::new(&config).await.unwrap();
    let mut app_ctx = AppContext::new(&config, infra);

    ratings::utils::warmup::warmup(&mut app_ctx).await.expect("Could not warm up database");
}

#[when(expr = "the client fetches the top snaps")]
async fn get_chart(world: &mut ChartWorld) {
    get_chart_internal(world, None).await;
}

#[when(expr = "the client fetches the top snaps for {category}")]
async fn get_chart_of_category(world: &mut ChartWorld, category: TestCategory) {
    get_chart_internal(world, Some(category.into())).await;
}

async fn get_chart_internal(world: &mut ChartWorld, category: Option<Category>) {
    world.chart_data = world
        .client
        .get_chart_of_category(Timeframe::Unspecified, category, &world.token)
        .await
        .expect("couldn't get chart")
        .into_inner()
        .ordered_chart_data;
}

#[then(expr = "the top {int} snaps are returned in the proper order")]
async fn chart_order(world: &mut ChartWorld, top: usize) {
    assert_eq!(world.chart_data.len(), top);

    assert!(world
        .chart_data
        .iter()
        .zip(world.snap_ids.iter())
        .all(|(data, id)| {
            let left = &data
                .rating
                .as_ref()
                .expect("no rating in chart data?")
                .snap_id;

            tracing::debug!("chart data: {data:?}, expected: {id}");

            left == id
        }))
}

#[then(expr = "the top snap returned is the one with the ID {string}")]
async fn check_test_snap(world: &mut ChartWorld, snap_id: String) {
    assert_eq!(
        world.test_snap, snap_id,
        "feature file and test snap definition got out of sync"
    );

    assert_eq!(
        &world.chart_data[0].rating.as_ref().unwrap().snap_id,
        &snap_id,
        "top chart result is not test snap"
    );
}

/// Automatically clears and snaps with >= TO_CLEAR votes, preventing them from interfering with tests
/// Being independent, while also not affecting other tests that require lower vote counts
async fn clear_db() {
    const TO_CLEAR: usize = 3;

    let config = Config::load().unwrap();
    let infra = Infrastructure::new(&config).await.unwrap();
    let mut conn = infra.repository().await.unwrap();

    let mut tx = conn.begin().await.unwrap();

    sqlx::query(
        r#"DELETE FROM votes WHERE snap_id IN
        (SELECT snap_id FROM votes GROUP BY snap_id HAVING COUNT(*) >= $1)
    "#,
    )
    .bind(TO_CLEAR as i64)
    .execute(&mut *tx)
    .await
    .unwrap();

    sqlx::query("TRUNCATE TABLE snap_categories")
        .execute(&mut *tx)
        .await
        .unwrap();

    tx.commit().await.unwrap();
}

#[tokio::main]
async fn main() {
    dotenvy::from_filename(".env_files/test.env").ok();

    ChartWorld::cucumber()
        .before(|_, _, _, _| clear_db().boxed_local())
        .repeat_failed()
        .max_concurrent_scenarios(1)
        .run_and_exit("tests/features/chart.feature")
        .await
}
