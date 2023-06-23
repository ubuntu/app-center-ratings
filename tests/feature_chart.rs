mod utils;

use pb::{ChartType, Timeframe};
use tonic::{metadata::MetadataValue, transport::Endpoint, Request, Response, Status};
use utils::{
    lifecycle::{after, before},
    server_url,
};

use crate::pb::{chart_client::ChartClient, GetChartRequest, GetChartResponse};

pub mod pb {
    tonic::include_proto!("ratings.feature.chart");
}

#[tokio::test]
async fn feature_chart() {
    before().await;

    let url = server_url();
    let channel = Endpoint::from_shared(url).unwrap().connect().await.unwrap();
    let mut client = ChartClient::with_interceptor(channel, with_auth);

    let body = GetChartRequest {
        timeframe: Timeframe::Week as i32,
        r#type: ChartType::Top as i32,
    };

    let request = tonic::Request::new(body);
    let response: Response<GetChartResponse> = client.get_chart(request).await.unwrap();
    let body = response.into_inner();

    let actual = body.ordered_chart_data.first().unwrap().app.as_str();
    let expected = "signal";
    assert_eq!(actual, expected);

    after().await;
}

fn with_auth(mut req: Request<()>) -> Result<Request<()>, Status> {
    let token: MetadataValue<_> = "Bearer foo".parse().unwrap();

    req.metadata_mut()
        .insert(http::header::AUTHORIZATION.as_str(), token.clone());

    Ok(req)
}
