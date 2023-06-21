use http::header;
use tonic::{Request, Status};
use tracing::info;

#[tracing::instrument]
pub fn require_auth(req: Request<()>) -> Result<Request<()>, Status> {
    info!("Validating request authorization");

    let Some(token) = req.metadata().get(header::AUTHORIZATION.as_str()) else {
        return Err(Status::unauthenticated("Missing authorization header"))
    };

    let token = token.to_str().unwrap_or("");

    if is_valid_token(token) {
        Ok(req)
    } else {
        Err(Status::unauthenticated("Invalid token"))
    }
}

fn is_valid_token(token: &str) -> bool {
    return true;
}
