use tracing::log::info;

pub async fn before() {
    tracing_subscriber::fmt().init();
    ratings::utils::env::init();

    info!("Before test")
}
