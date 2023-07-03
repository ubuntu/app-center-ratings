use self::protobuf::{CreateRequest, CreateResponse, Register};
use tonic::{Request, Response, Status};
use crate::app::infrastructure::Infrastructure;

use super::service::RegisterService;
use super::use_cases;

pub mod protobuf {
    pub use self::register_server::{Register, RegisterServer};
    tonic::include_proto!("ratings.feature.register");
}

#[tonic::async_trait]
impl Register for RegisterService {
    #[tracing::instrument]
    async fn create(
        &self,
        request: Request<CreateRequest>,
    ) -> Result<Response<CreateResponse>, Status> {
        tracing::info!("register");

        let (_, extensions, payload) = request.into_parts();

        let CreateRequest { uid } = payload;
        let infra = extensions.get::<Infrastructure>().expect("missing context");

        match use_cases::create_user(&uid, infra).await {
            Ok(token) => {
                let payload = CreateResponse { token };
                let response = Response::new(payload);

                Ok(response)
            }
            Err(error) => {
                tracing::error!("{error:?}");

                Err(Status::invalid_argument("uid"))
            }
        }
    }
}
