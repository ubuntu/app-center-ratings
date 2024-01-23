use tracing_subscriber::EnvFilter;

mod app;
mod features;
mod utils;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = utils::Config::load()?;

    let app_name = config.name.as_str();
    let app_log_level = config.log_level.as_str();
    let app_logging_directive = format!("{app_name}={app_log_level}").parse()?;
    let max_level = EnvFilter::from_default_env().add_directive(app_logging_directive);

    tracing_subscriber::fmt()
        .with_env_filter(max_level)
        .with_span_events(tracing_subscriber::fmt::format::FmtSpan::CLOSE)
        .init();

    tracing::info!("Starting the Ubuntu App Rating Service");
    app::run(config).await?;

    Ok(())
}
