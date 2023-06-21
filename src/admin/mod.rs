use axum::{routing::get, Router};

pub fn build_admin_router() -> Router {
    Router::new().route("/", get(healthz))
}

async fn healthz() -> &'static str {
    "OK"
}
