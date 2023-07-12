use tracing::info;

use super::infrastructure;

pub async fn before() {
    tracing_subscriber::fmt().init();
    ratings::utils::env::init();
    infrastructure::init().await;

    info!("Before test")
}

pub async fn after() {
    info!("After test")
}
