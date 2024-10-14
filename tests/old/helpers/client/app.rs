use tonic::async_trait;
use tonic::{metadata::MetadataValue, transport::Endpoint, Request, Response, Status};

use ratings::features::pb::app::{GetRatingRequest, GetRatingResponse};

use ratings::features::pb::app::app_client as pb;

use super::Client;

#[async_trait]
pub trait AppClient: Client {
    async fn get_rating(
        &self,
        token: &str,
        id: &str,
    ) -> Result<Response<GetRatingResponse>, Status> {
        let channel = Endpoint::from_shared(self.url().to_string())
            .unwrap()
            .connect()
            .await
            .unwrap();
        let mut client = pb::AppClient::with_interceptor(channel, move |mut req: Request<()>| {
            let header: MetadataValue<_> = format!("Bearer {token}").parse().unwrap();
            req.metadata_mut().insert("authorization", header);
            Ok(req)
        });
        client
            .get_rating(GetRatingRequest {
                snap_id: id.to_string(),
            })
            .await
    }
}
