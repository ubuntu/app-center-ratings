mod interface;
mod service;

use axum::{routing::get, Router};
use interface::vote;

pub fn build_vote_router() -> Router {
    Router::new().route("/v1/vote", get(vote))
}
