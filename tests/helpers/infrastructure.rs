use std::sync::Arc;

use once_cell::sync::OnceCell;
use sqlx::pool::PoolConnection;
use sqlx::postgres::PgPoolOptions;
use sqlx::Postgres;

use ratings::utils::env;
use ratings::utils::infrastructure::Infrastructure;
use ratings::utils::jwt::Jwt;

pub static TEST_INFRA: OnceCell<Infrastructure> = OnceCell::new();

pub async fn init() {
    let uri = env::get_config().postgres_uri.clone();
    let uri = uri.as_str();

    let postgres = PgPoolOptions::new()
        .max_connections(5)
        .connect(uri)
        .await
        .unwrap();
    let postgres = Arc::new(postgres);
    let jwt = Jwt::new();
    let infra = Infrastructure { postgres, jwt };
    TEST_INFRA
        .set(infra)
        .expect("infrastructure should be initialised");
}

pub async fn get_repository() -> PoolConnection<Postgres> {
    let infra = TEST_INFRA
        .get()
        .expect("infrastructure should be initialised");
    infra.postgres.acquire().await.unwrap()
}
