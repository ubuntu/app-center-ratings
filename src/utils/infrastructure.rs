use std::{
    error::Error,
    fmt::{Debug, Formatter},
    sync::Arc,
};

use sqlx::{pool::PoolConnection, postgres::PgPoolOptions, PgPool, Postgres};

use crate::utils::{config::Config, jwt::Jwt};

#[derive(Clone)]
pub struct Infrastructure {
    pub postgres: Arc<PgPool>,
    pub jwt: Arc<Jwt>,
}

impl Infrastructure {
    pub async fn new(config: &Config) -> Result<Infrastructure, Box<dyn Error>> {
        let uri = config.postgres_uri.clone();
        let uri = uri.as_str();

        let postgres = PgPoolOptions::new().max_connections(5).connect(uri).await?;
        let postgres = Arc::new(postgres);

        let jwt = Jwt::new(&config.jwt_secret)?;
        let jwt = Arc::new(jwt);

        Ok(Infrastructure { postgres, jwt })
    }

    pub async fn repository(&self) -> Result<PoolConnection<Postgres>, sqlx::Error> {
        self.postgres.acquire().await
    }
}

impl Debug for Infrastructure {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("Infrastructure { postgres, jwt }")
    }
}
