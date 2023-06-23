use self::pb::{RegisterRequest, RegisterResponse};
use tonic::{Request, Response, Status};

pub mod pb {
    tonic::include_proto!("ratings.feature.user_no_auth");
}

pub use pb::user_no_auth_server;

#[derive(Default, Debug)]
pub struct UserNoAuthService;

#[tonic::async_trait]
impl pb::user_no_auth_server::UserNoAuth for UserNoAuthService {
    #[tracing::instrument]
    async fn register(
        &self,
        request: Request<RegisterRequest>,
    ) -> Result<Response<RegisterResponse>, Status> {
        tracing::info!("");
        let request = request.into_inner();

        if request.uid.is_empty() {
            return Err(Status::invalid_argument("Invalid uid"));
        }

        let payload = RegisterResponse {
            token: "uid-token".to_string(),
        };

        Ok(Response::new(payload))
    }
}
