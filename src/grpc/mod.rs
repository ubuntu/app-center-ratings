use crate::{
    db,
    jwt::JwtVerifier,
    middleware::AuthLayer,
    proto::common::{ChartData as PbChartData, Rating as PbRating},
    ratings::{get_snap_name, ChartData, Rating},
    Context,
};
use futures::future::try_join_all;
use std::{error::Error, fs::read_to_string, net::SocketAddr, sync::Arc};
use tonic::{
    transport::{Identity, Server, ServerTlsConfig},
    Status,
};
use tracing::{error, warn};
mod app;
mod charts;
mod user;

use app::RatingService;
use charts::ChartService;
use user::UserService;

impl From<db::Error> for Status {
    fn from(value: db::Error) -> Self {
        Status::internal(value.to_string())
    }
}

pub async fn run_server(ctx: Context) -> Result<(), Box<dyn std::error::Error>> {
    let verifier = JwtVerifier::from_secret(&ctx.config.jwt_secret)?;
    let addr: SocketAddr = ctx.config.socket().parse()?;

    let keychain_path = ctx.config.tls_keychain_path.clone();
    let key_path = ctx.config.tls_key_path.clone();

    let builder = match (keychain_path, key_path) {
        (Some(keychain_path), Some(key_path)) => {
            let keychain = read_to_string(keychain_path)?;
            let key = read_to_string(key_path)?;
            let identity = Identity::from_pem(keychain, key);
            Server::builder().tls_config(ServerTlsConfig::new().identity(identity))?
        }
        (Some(_), None) | (None, Some(_)) => {
            panic!("Both TLS keychain and private key must be provided, or neither.");
        }
        (None, None) => {
            warn!("TLS is not configured as the environment variables are not set.");
            Server::builder()
        }
    };

    let ctx = Arc::new(ctx);

    builder
        .layer(AuthLayer::new(verifier))
        .add_service(RatingService::new_server(ctx.clone()))
        .add_service(ChartService::new_server(ctx.clone()))
        .add_service(UserService::new_server(ctx.clone()))
        .serve(addr)
        .await?;

    Ok(())
}

pub(crate) async fn populate_chart_data_with_names(
    ctx: &Arc<Context>,
    chart_data_vec: Vec<ChartData>,
) -> Result<Vec<PbChartData>, Status> {
    try_join_all(chart_data_vec.into_iter().map(|chart_data| async {
        let snap_name = get_snap_name(
            &chart_data.rating.snap_id,
            &ctx.config.snapcraft_io_uri,
            &ctx.http_client,
        )
        .await
        .map_err(|e| {
            let mut err = &e as &dyn Error;
            let mut error_chain = format!("{err}");
            while let Some(src) = err.source() {
                error_chain.push_str(&format!("\nCaused by: {src}"));
                err = src;
            }
            error!(error=%error_chain, "unable to fetch snap name");
            Status::unknown("Internal server error")
        })?;

        Ok(PbChartData::from_chart_data_and_snap_name(
            chart_data, snap_name,
        ))
    }))
    .await
}

impl PbChartData {
    fn from_chart_data_and_snap_name(chart_data: ChartData, snap_name: String) -> Self {
        Self {
            raw_rating: chart_data.raw_rating,
            rating: Some(PbRating::from_rating_and_snap_name(
                chart_data.rating,
                snap_name,
            )),
        }
    }
}

impl PbRating {
    fn from_rating_and_snap_name(rating: Rating, snap_name: String) -> Self {
        Self {
            snap_id: rating.snap_id,
            total_votes: rating.total_votes,
            ratings_band: rating.ratings_band as i32,
            snap_name,
        }
    }
}
