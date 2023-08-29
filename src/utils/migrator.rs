use std::error::Error;
use std::fmt::{Debug, Formatter};
use std::sync::Arc;

use sqlx::{postgres::PgPoolOptions, PgPool};

const MIGRATIONS_PATH: &str = "sql/migrations";

#[derive(Clone)]
pub struct Migrator {
    pub pool: Arc<PgPool>,
}

#[allow(dead_code)]
impl Migrator {
    pub async fn new(uri: &str) -> Result<Migrator, Box<dyn Error>> {
        let pool = PgPoolOptions::new().max_connections(1).connect(uri).await?;
        let pool = Arc::new(pool);
        Ok(Migrator { pool })
    }

    pub async fn run(&self) -> Result<(), sqlx::Error> {
        let m = sqlx::migrate::Migrator::new(std::path::Path::new(MIGRATIONS_PATH)).await?;

        m.run(&mut self.pool.acquire().await?).await?;
        Ok(())
    }

    pub async fn revert(&self) -> Result<(), sqlx::Error> {
        let m = sqlx::migrate::Migrator::new(std::path::Path::new(MIGRATIONS_PATH)).await?;

        m.undo(&mut self.pool.acquire().await?, 1).await?;

        Ok(())
    }
}

impl Debug for Migrator {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("Migrator { migrations_pool }")
    }
}
