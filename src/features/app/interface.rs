use crate::app::AppContext;

use crate::features::pb::app::{GetRatingRequest, GetRatingResponse};
use tonic::{Request, Response, Status};

use crate::features::pb::app::app_server::App;

use super::{service::AppService, use_cases};

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
