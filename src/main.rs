mod app;
mod features;
mod utils;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    utils::env::init();
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_span_events(tracing_subscriber::fmt::format::FmtSpan::CLOSE)
        .init();
    tracing::info!("Starting Ubuntu App Rating Service");
    utils::infrastructure::init().await;
    app::build_and_run().await;

    Ok(())
}
