use crate::utils::server_url;
use protobuf::{CreateRequest, CreateResponse, RegisterClient};
use tonic::{Response, Status};

pub mod protobuf {
    pub use self::register_client::RegisterClient;
    tonic::include_proto!("ratings.feature.register");
}

pub async fn do_create(uid: &str) -> Result<Response<CreateResponse>, Status> {
    let url = server_url();
    let mut client = RegisterClient::connect(url).await.unwrap();
    client
        .create(CreateRequest {
            uid: uid.to_string(),
        })
        .await
}
