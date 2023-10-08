use std::env;

use tracing::info;

mod app;
mod features;
mod utils;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = utils::Config::load()?;

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_span_events(tracing_subscriber::fmt::format::FmtSpan::CLOSE)
        .init();

    tracing::info!("Starting THE Ubuntu App Rating Service");

    match env::current_dir() {
        Ok(cur_dir) => info!("Current directory: {}", cur_dir.display()),
        Err(e) => info!("Error retrieving current directory: {:?}", e),
    }
    app::run(config).await?;

    Ok(())
}
