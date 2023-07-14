use tonic::metadata::MetadataValue;
use tonic::transport::Endpoint;
use tonic::{Request, Response, Status};

use crate::helpers::env::get_server_base_url;

pub mod pb {
    pub use self::user_client::UserClient;

    tonic::include_proto!("ratings.features.user");
}

#[derive(Debug, Clone)]
pub struct UserClient {
    url: String,
}

impl UserClient {
    pub fn new() -> Self {
        Self {
            url: get_server_base_url(),
        }
    }

    pub async fn register(&self, id: &str) -> Result<Response<pb::RegisterResponse>, Status> {
        let mut client = pb::UserClient::connect(self.url.clone()).await.unwrap();
        client
            .register(pb::RegisterRequest { id: id.to_string() })
            .await
    }

    pub async fn authenticate(
        &self,
        id: &str,
    ) -> Result<Response<pb::AuthenticateResponse>, Status> {
        let mut client = pb::UserClient::connect(self.url.clone()).await.unwrap();
        client
            .authenticate(pb::AuthenticateRequest { id: id.to_string() })
            .await
    }

    pub async fn vote(&self, token: &str, ballet: pb::VoteRequest) -> Result<Response<()>, Status> {
        let channel = Endpoint::from_shared(self.url.clone())
            .unwrap()
            .connect()
            .await
            .unwrap();
        let mut client = pb::UserClient::with_interceptor(channel, move |mut req: Request<()>| {
            let header: MetadataValue<_> = format!("Bearer {token}").parse().unwrap();
            req.metadata_mut().insert("authorization", header.clone());
            Ok(req)
        });
        client.vote(ballet).await
    }

    pub async fn delete(&self, token: &str) -> Result<Response<()>, Status> {
        let channel = Endpoint::from_shared(self.url.clone())
            .unwrap()
            .connect()
            .await
            .unwrap();
        let mut client = pb::UserClient::with_interceptor(channel, move |mut req: Request<()>| {
            let header: MetadataValue<_> = format!("Bearer {token}").parse().unwrap();
            req.metadata_mut().insert("authorization", header.clone());
            Ok(req)
        });

        client.delete(()).await
    }
}
