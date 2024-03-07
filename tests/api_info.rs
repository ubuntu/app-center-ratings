use cucumber::{given, then, when, World};
use lazy_static::lazy_static;
use ratings::{features::admin::api_version::ApiVersion, utils::Config};
use regex::Regex;

use helpers::client::*;

mod helpers;

#[derive(Clone, Debug, World)]
#[world(init = Self::new)]
struct LogWorld {
    client: TestClient,
    returned_info: Option<ApiVersion<'static>>,
}

impl LogWorld {
    fn new() -> Self {
        let config = Config::load().expect("could not load config");
        let client = TestClient::new(config.socket());
        Self {
            client,
            returned_info: None,
        }
    }
}

#[given(expr = "Big doesn't know the API build info")]
fn unknown_level(world: &mut LogWorld) {
    world.returned_info = None
}

#[when(expr = "Big asks for the API info")]
async fn get_api_info(world: &mut LogWorld) {
    world.returned_info = Some(
        world
            .client
            .get_api_info()
            .await
            .expect("could not get API info")
            .0,
    )
}

lazy_static! {
    static ref VALID_SHA: Regex = Regex::new(r"/^([a-f0-9]{64})$/").unwrap();
    static ref VALID_SEMVER: Regex = Regex::new(r"((\d+).?){0,3}").unwrap();
}

#[then(expr = "Big gets an answer")]
fn got_info(world: &mut LogWorld) {
    assert!(
        world.returned_info.is_some(),
        "did not get a valid level from the endpoint"
    );

    let info = world.returned_info.as_ref().unwrap();
    VALID_SHA.is_match(&info.commit);
    VALID_SEMVER.is_match(&info.version);
    // The regex for a valid git branch is too absurd to even bother testing,
    // and also may not even be present (e.g. build from a detached HEAD).
}

#[tokio::main]
async fn main() {
    dotenvy::from_filename(".env_files/test.env").ok();

    LogWorld::cucumber()
        .repeat_skipped()
        .init_tracing()
        .max_concurrent_scenarios(1)
        .run_and_exit("tests/features/admin/api-info.feature")
        .await
}
