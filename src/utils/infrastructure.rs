use std::fmt::{Debug, Formatter};
use std::sync::Arc;

use once_cell::sync::OnceCell;
use sqlx::pool::PoolConnection;
use sqlx::{postgres::PgPoolOptions, PgPool, Postgres};

use crate::utils::env;
use crate::utils::jwt::Jwt;

pub static INFRA: OnceCell<Infrastructure> = OnceCell::new();

pub struct Infrastructure {
    pub postgres: Arc<PgPool>,
    pub jwt: Jwt,
}

impl Debug for Infrastructure {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("Infrastructure { postgres, jwt }")
    }
}

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
    INFRA
        .set(infra)
        .expect("infrastructure should be initialised");
}

pub async fn get_repository() -> Result<PoolConnection<Postgres>, sqlx::Error> {
    let infra = INFRA.get().expect("infrastructure should be initialised");
    infra.postgres.acquire().await
}
