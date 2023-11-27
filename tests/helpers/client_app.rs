use tonic::{metadata::MetadataValue, transport::Endpoint, Request, Response, Status};

use crate::pb::app::app_client as pb;
use crate::pb::app::{GetRatingRequest, GetRatingResponse};

#[derive(Debug, Clone)]
pub struct AppClient {
    url: String,
}

impl AppClient {
    pub fn new(socket: &str) -> Self {
        Self {
            url: format!("http://{socket}/"),
        }
    }

    pub async fn get_rating(
        &self,
        id: &str,
        token: &str,
    ) -> Result<Response<GetRatingResponse>, Status> {
        let channel = Endpoint::from_shared(self.url.clone())
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
