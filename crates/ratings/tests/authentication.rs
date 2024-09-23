use cucumber::{given, then, when, World};

use helpers::client::*;
use ratings::utils::{Config, Infrastructure};
use sqlx::Row;
use tonic::{Code, Status};

mod helpers;

#[derive(Clone, Debug, Default, World)]
struct AuthenticationWorld {
    client_hash: String,
    client: Option<TestClient>,
    tokens: Vec<String>,
    auth_error: Option<Status>,
}

#[given(expr = "a valid client hash")]
fn generate_hash(world: &mut AuthenticationWorld) {
    world.client_hash = helpers::data_faker::rnd_sha_256();
}

#[given(expr = "a bad client with the hash {word}")]
fn with_hash(world: &mut AuthenticationWorld, hash: String) {
    world.client_hash = hash;
}

#[when(expr = "the client attempts to authenticate")]
#[when(expr = "that client authenticates a second time")]
#[given(expr = "an authenticated client")]
async fn authenticate(world: &mut AuthenticationWorld) {
    let config = Config::load().expect("Could not load config");

    world.client = Some(TestClient::new(config.socket()));

    match world
        .client
        .as_ref()
        .unwrap()
        .authenticate(&world.client_hash)
        .await
    {
        Ok(resp) => world.tokens.push(resp.into_inner().token),
        Err(err) => world.auth_error = Some(err),
    }
}

#[then(expr = "the authentication is rejected")]
fn check_rejected(world: &mut AuthenticationWorld) {
    assert!(world.auth_error.is_some());

    let err = world.auth_error.as_ref().unwrap();

    assert_eq!(err.code(), Code::InvalidArgument);
}

#[then(expr = "the returned token is valid")]
#[then(expr = "both tokens are valid")]
fn verify_token(world: &mut AuthenticationWorld) {
    assert!(
        world.auth_error.is_none(),
        "needed clean exit, instead got status {:?}",
        world.auth_error
    );

    for token in world.tokens.iter() {
        helpers::assert::assert_token_is_valid(token);
    }
}

#[then(expr = "the hash is only in the database once")]
async fn no_double_auth(world: &mut AuthenticationWorld) {
    // In other test scenarios we might do this when we init the world, but
    // given authentication only needs this once this is fine
    let config = Config::load().expect("Could not load config");
    let infra = Infrastructure::new(&config)
        .await
        .expect("Could not init DB");

    // User still registered
    let row = sqlx::query("SELECT COUNT(*) FROM users WHERE client_hash = $1")
        .bind(&world.client_hash)
        .fetch_one(&mut *infra.repository().await.expect("could not connect to DB"))
        .await
        .unwrap();

    let count: i64 = row.try_get("count").expect("Failed to get count");

    // Only appears in db once
    assert_eq!(count, 1);
}

#[tokio::main]
async fn main() {
    dotenvy::from_filename(".env_files/test.env").ok();

    AuthenticationWorld::cucumber()
        .repeat_skipped()
        .run_and_exit("tests/features/user/authentication.feature")
        .await
}
