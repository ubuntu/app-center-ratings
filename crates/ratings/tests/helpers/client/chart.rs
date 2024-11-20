use tonic::metadata::MetadataValue;
use tonic::transport::Endpoint;
use tonic::{async_trait, Request, Response, Status};

use ratings::features::pb::chart::{chart_client as pb, Category, Timeframe};
use ratings::features::pb::chart::{GetChartRequest, GetChartResponse};

use super::Client;

#[async_trait]
pub trait ChartClient: Client {
    async fn get_chart(
        &self,
        timeframe: Timeframe,
        token: &str,
    ) -> Result<Response<GetChartResponse>, Status> {
        let channel = Endpoint::from_shared(self.url().to_string())
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
            .get_chart(GetChartRequest {
                timeframe: timeframe.into(),
                category: None,
            })
            .await
    }

    async fn get_chart_of_category(
        &self,
        timeframe: Timeframe,
        category: Option<Category>,
        token: &str,
    ) -> Result<Response<GetChartResponse>, Status> {
        let channel = Endpoint::from_shared(self.url().to_string())
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
            .get_chart(GetChartRequest {
                timeframe: timeframe.into(),
                category: category.map(|v| v.into()),
            })
            .await
    }
}
