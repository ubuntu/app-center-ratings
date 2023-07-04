use tonic::{Response, Status};

use protobuf::{CreateRequest, CreateResponse, RegisterClient as GrpcClient};

use crate::utils::server_url;

pub mod protobuf {
    pub use self::register_client::RegisterClient;

    tonic::include_proto!("ratings.feature.register");
}

#[derive(Default)]
pub struct RegisterClient;

impl RegisterClient {
    pub async fn register(&self, uid: &str) -> Result<Response<CreateResponse>, Status> {
        let url = server_url();
        let mut client = GrpcClient::connect(url).await.unwrap();
        client
            .create(CreateRequest {
                uid: uid.to_string(),
            })
            .await
    }
}
