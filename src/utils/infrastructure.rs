use std::fmt::{Debug, Formatter};
use std::sync::Arc;

use once_cell::sync::OnceCell;
use sqlx::{Pool, Postgres, postgres::PgPoolOptions};

use crate::utils::env;
use crate::utils::jwt::Jwt;

pub struct Infrastructure {
    pub postgres: Arc<Pool<Postgres>>,
    pub jwt: Jwt,
}

impl Debug for Infrastructure {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("Infrastructure { postgres, jwt }")
    }
}

impl Infrastructure {
    pub async fn new() -> Self {
        let uri = env::get_postgres_uri();

        let postgres = PgPoolOptions::new()
            .max_connections(5)
            .connect(uri.as_str())
            .await
            .unwrap();
        let postgres = Arc::new(postgres);
        let jwt = Jwt::new();

        Infrastructure { postgres, jwt }
    }
}
