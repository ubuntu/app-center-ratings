use crate::app::AppContext;

use self::protobuf::{App, GetRatingRequest, GetRatingResponse};
pub use protobuf::app_server;
use tonic::{Request, Response, Status};

use super::{service::AppService, use_cases};

pub mod protobuf {
    pub use self::app_server::App;
    tonic::include_proto!("ratings.features.common");
    tonic::include_proto!("ratings.features.app");
}

#[tonic::async_trait]
impl App for AppService {
    #[tracing::instrument]
    async fn get_rating(
        &self,
        request: Request<GetRatingRequest>,
    ) -> Result<Response<GetRatingResponse>, Status> {
        let app_ctx = request.extensions().get::<AppContext>().unwrap().clone();

        let GetRatingRequest { snap_id } = request.into_inner();

        if snap_id.is_empty() {
            return Err(Status::invalid_argument("snap id"));
        }

        let result = use_cases::get_rating(&app_ctx, snap_id).await;

        match result {
            Ok(rating) => {
                let payload = GetRatingResponse {
                    rating: Some(rating.into_dto()),
                };
                Ok(Response::new(payload))
            }
            Err(_error) => Err(Status::unknown("Internal server error")),
        }
    }
}
