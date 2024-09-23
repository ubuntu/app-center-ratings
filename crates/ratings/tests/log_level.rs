use std::str::FromStr;

use cucumber::{given, then, when, Parameter, World};

use helpers::client::*;
use ratings::utils::Config;

mod helpers;

#[derive(Copy, Clone, Eq, PartialEq, Parameter, Debug)]
#[param(name = "level", regex = "info|warn|debug|trace|error")]
pub struct Level(log::Level);

impl FromStr for Level {
    type Err = <log::Level as FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Level(log::Level::from_str(s)?))
    }
}

impl From<log::Level> for Level {
    fn from(value: log::Level) -> Self {
        Self(value)
    }
}

impl From<Level> for log::Level {
    fn from(value: Level) -> Self {
        value.0
    }
}

#[derive(Clone, Debug, World)]
#[world(init = Self::new)]
struct LogWorld {
    client: TestClient,
    current_level: Option<Level>,
}

impl LogWorld {
    fn new() -> Self {
        let config = Config::load().expect("could not load config");
        let client = TestClient::new(config.socket());
        Self {
            client,
            current_level: None,
        }
    }
}

#[given(expr = "Espio doesn't know the log level")]
fn unknown_level(world: &mut LogWorld) {
    world.current_level = None
}

#[when(expr = "Espio asks for the log level")]
#[given(expr = "the service's current log level")]
async fn get_log_level(world: &mut LogWorld) {
    world.current_level = Some(
        world
            .client
            .get_log_level()
            .await
            .expect("could not get log level")
            .level
            .into(),
    )
}

#[when(expr = "Espio requests it changes to {level}")]
async fn set_log_level(world: &mut LogWorld, level: Level) {
    world
        .client
        .set_log_level(level.into())
        .await
        .expect("problem setting log level");
}

#[then(expr = "Espio gets an answer")]
fn got_any_level(world: &mut LogWorld) {
    assert!(
        world.current_level.is_some(),
        "did not get a valid level from the endpoint"
    );
}

#[then(expr = "the log level is set to {level}")]
async fn got_expected_level(world: &mut LogWorld, level: Level) {
    let post_set_level = world
        .client
        .get_log_level()
        .await
        .expect("could not get log level")
        .level;

    assert_eq!(level.0, post_set_level)
}

#[tokio::main]
async fn main() {
    dotenvy::from_filename(".env_files/test.env").ok();

    LogWorld::cucumber()
        .repeat_skipped()
        .max_concurrent_scenarios(1)
        .run_and_exit("tests/features/admin/log-level.feature")
        .await
}
