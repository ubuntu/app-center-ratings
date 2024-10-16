//! Resources for database migration
use std::{
    env,
    error::Error,
    fmt::{Debug, Formatter},
    sync::Arc,
};

use sqlx::{postgres::PgPoolOptions, PgPool};
use tracing::info;

/// The path for the migration schematics
const MIGRATIONS_PATH: &str = "./sql/migrations";

/// A wrapper for a postgres pool for doing migrations
#[derive(Clone)]
pub struct Migrator {
    /// A connection pool to a postgres resource.
    pub pool: Arc<PgPool>,
}

impl Migrator {
    /// Attempts to create a new Migration object from the given resource URI.
    pub async fn new(uri: &str) -> Result<Migrator, Box<dyn Error>> {
        let pool = PgPoolOptions::new().max_connections(1).connect(uri).await?;
        let pool = Arc::new(pool);

        Ok(Migrator { pool })
    }

    /// Get the paths for migration backups and templates
    fn migrations_path() -> String {
        let snap_path = std::env::var("SNAP").unwrap_or("./sql".to_string());
        format!("{}/migrations", snap_path)
    }

    /// Runs a database migration as specified by the URI given on input.
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

    /// Attempts to revert a migration to a previous good state.
    #[allow(dead_code)]
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
