use axum::{
    extract::Path,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Extension, Router,
};
use serde_json::json;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

const PORT: u16 = 11111;

type State = Arc<Mutex<StateInner>>;

#[derive(Default, Debug)]
pub struct StateInner {
    id_map: HashMap<String, String>, // id -> name
    categories: HashMap<String, Vec<String>>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .compact()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let app = Router::new()
        // mocked snapcraft.io endpoints
        .route(
            "/assertions/snap-declaration/16/:snap_id",
            get(snap_assertions),
        )
        .route("/snaps/info/:snap_name", get(snap_info))
        // admin endpoint
        .route("/__admin__/register-snap/:snap_id", post(register_snap))
        .layer(Extension(State::default()));

    info!("Starting mock-server");
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{PORT}"))
        .await
        .unwrap();

    axum::serve(listener, app).await.unwrap();
}

async fn register_snap(
    Path(snap_id): Path<String>,
    Extension(state): Extension<State>,
    categories: String,
) -> impl IntoResponse {
    info!("registering snap: {snap_id} -> {categories:?}");
    let categories: Vec<String> = categories.split(',').map(|c| c.to_string()).collect();
    let snap_name = Uuid::new_v4().to_string();

    let mut guard = state.lock().unwrap();
    guard.id_map.insert(snap_id, snap_name.clone());
    guard.categories.insert(snap_name, categories);

    (StatusCode::OK, "registered")
}

async fn snap_assertions(
    Path(snap_id): Path<String>,
    Extension(state): Extension<State>,
) -> impl IntoResponse {
    info!("getting snap assertions for {snap_id}");
    let guard = state.lock().unwrap();

    match guard.id_map.get(&snap_id) {
        Some(name) => (
            StatusCode::OK,
            json!({ "headers": { "snap-name": name } }).to_string(),
        ),

        None => {
            warn!("attempt to pull snap name for unknown id: {snap_id}");
            (
                StatusCode::NOT_FOUND,
                json!({ "error": "not found" }).to_string(),
            )
        }
    }
}

async fn snap_info(
    Path(snap_name): Path<String>,
    Extension(state): Extension<State>,
) -> impl IntoResponse {
    info!("getting categories for {snap_name}");
    let guard = state.lock().unwrap();

    match guard.categories.get(&snap_name) {
        Some(cats) => {
            let categories: Vec<_> = cats.iter().map(|c| json!({ "name": c })).collect();
            (
                StatusCode::OK,
                json!({ "snap": { "categories": categories } }).to_string(),
            )
        }

        None => {
            warn!("attempt to pull snap categories for unknown snap: {snap_name}");
            (
                StatusCode::NOT_FOUND,
                json!({ "error": "not found" }).to_string(),
            )
        }
    }
}
