use std::{
    env,
    error::Error,
    fmt::{Debug, Formatter},
    sync::Arc,
};

use sqlx::{postgres::PgPoolOptions, PgPool};
use tracing::info;

const MIGRATIONS_PATH: &str = "./sql/migrations";

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

    fn migrations_path() -> String {
        let snap_path = std::env::var("SNAP").unwrap_or("./sql".to_string());
        format!("{}/migrations", snap_path)
    }

    pub async fn run(&self) -> Result<(), sqlx::Error> {
        match env::current_dir() {
            Ok(cur_dir) => info!("Current directory: {}", cur_dir.display()),
            Err(e) => info!("Error retrieving current directory: {:?}", e),
        }
        let m =
            sqlx::migrate::Migrator::new(std::path::Path::new(&Self::migrations_path())).await?;

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
