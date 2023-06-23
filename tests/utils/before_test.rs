use tracing::log::info;

pub async fn before() {
    tracing_subscriber::fmt().init();

    info!("Before test")
}
