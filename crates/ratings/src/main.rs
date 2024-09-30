use ratings::{app, utils};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = utils::Config::load()?;

    app::run(config).await?;

    Ok(())
}
