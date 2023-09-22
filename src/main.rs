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

    tracing::info!("Starting Ubuntu App Rating Service");

    tracing::info!("Config I have: {:?}", config);
    tracing::info!("I have been update again, yippie!");

    app::run(config).await?;

    Ok(())
}
