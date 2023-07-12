use tracing::log::info;

use crate::utils::infra;

pub async fn before() {
    tracing_subscriber::fmt().init();
    ratings::utils::env::init();
    infra::init().await;

    info!("Before test")
}
