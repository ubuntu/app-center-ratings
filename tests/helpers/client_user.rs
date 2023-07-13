use tonic::metadata::MetadataValue;
use tonic::transport::Endpoint;
use tonic::{Request, Response, Status};

use protobuf::UserClient as GrpcClient;
pub use protobuf::{LoginRequest, LoginResponse};

use crate::helpers::env::get_server_base_url;

pub mod protobuf {
    pub use self::user_client::UserClient;

    tonic::include_proto!("ratings.features.user");
}

pub struct UserClient {
    url: String,
}

impl UserClient {
    pub fn new() -> Self {
        Self {
            url: get_server_base_url(),
        }
    }

    pub async fn login(&self, user_id: &str) -> Result<Response<LoginResponse>, Status> {
        let mut client = GrpcClient::connect(self.url.clone()).await.unwrap();
        client
            .login(LoginRequest {
                user_id: user_id.to_string(),
            })
            .await
    }

    pub async fn delete(&self, token: &str) -> Result<Response<()>, Status> {
        let channel = Endpoint::from_shared(self.url.clone())
            .unwrap()
            .connect()
            .await
            .unwrap();
        let mut client = GrpcClient::with_interceptor(channel, move |mut req: Request<()>| {
            let header: MetadataValue<_> = format!("Bearer {token}").parse().unwrap();
            req.metadata_mut().insert("authorization", header.clone());
            Ok(req)
        });

        client.delete(()).await
    }
}
