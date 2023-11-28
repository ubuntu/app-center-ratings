use tonic::metadata::MetadataValue;
use tonic::transport::Endpoint;
use tonic::{Request, Response, Status};

use crate::pb::chart::{chart_client as pb, Timeframe};
use crate::pb::chart::{
    GetChartRequest, GetChartResponse
};

#[derive(Debug, Clone)]
pub struct ChartClient {
    url: String,
}

impl ChartClient {
    pub fn new(socket: &str) -> Self {
        Self {
            url: format!("http://{socket}/"),
        }
    }

    pub async fn get_chart(
        &self,
        timeframe: Timeframe,
        token: &str,
    ) -> Result<Response<GetChartResponse>, Status> {
        let channel = Endpoint::from_shared(self.url.clone())
            .unwrap()
            .connect()
            .await
            .unwrap();
        let mut client = pb::ChartClient::with_interceptor(channel, move |mut req: Request<()>| {
            let header: MetadataValue<_> = format!("Bearer {token}").parse().unwrap();
            req.metadata_mut().insert("authorization", header);
            Ok(req)
        });
        client
            .get_chart(GetChartRequest{
                timeframe: timeframe.into(),
            })
            .await
    }
}
