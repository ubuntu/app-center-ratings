use std::sync::Arc;

use sqlx::{postgres::PgPoolOptions, Pool, Postgres};

use crate::utils::env;

#[derive(Debug)]
pub struct Infrastructure {
    pub postgres: Arc<Pool<Postgres>>,
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

        Infrastructure { postgres }
    }
}
