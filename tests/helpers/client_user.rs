use tonic::{Response, Status};

use protobuf::{LoginRequest, LoginResponse, UserClient as GrpcClient};
use ratings::utils::env::get_socket;

pub mod protobuf {
    pub use self::user_client::UserClient;

    tonic::include_proto!("ratings.features.user");
}

#[derive(Default)]
pub struct UserClient;

impl UserClient {
    pub async fn login(&self, uid: &str) -> Result<Response<LoginResponse>, Status> {
        let url = get_socket();
        let mut client = GrpcClient::connect(url).await.unwrap();
        client
            .login(LoginRequest {
                uid: uid.to_string(),
            })
            .await
    }
}
