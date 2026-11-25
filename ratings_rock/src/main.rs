use ratings::{db::check_db_conn, grpc::run_server, Config, Context};
use std::io::stdout;
use tracing::{info, subscriber::set_global_default};
use tracing_subscriber::{EnvFilter, FmtSubscriber};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(stdout)
        .json()
        .flatten_event(true)
        .with_span_list(true)
        .with_current_span(false)
        .with_file(true)
        .with_line_number(true)
        .with_thread_ids(true)
        .finish();

    set_global_default(subscriber).expect("unable to set a global tracing subscriber");

    info!("loading application context");
    let ctx = Context::new(Config::load()?)?;

    info!("checking DB connectivity");
    check_db_conn().await?; // Ensure that the migrations run before server start

    info!("starting server");
    run_server(ctx).await?;

    Ok(())
}
