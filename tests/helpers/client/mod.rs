pub mod app;
pub mod chart;
pub mod log_level;
pub mod user;

use std::fmt::Display;

pub use self::{app::AppClient, chart::ChartClient, log_level::LogClient, user::UserClient};

pub trait Client {
    fn url(&self) -> &str;
}

#[derive(Debug, Clone)]
pub struct TestClient {
    url: String,
}

impl TestClient {
    pub fn new<D: Display>(url: D) -> Self {
        Self {
            url: format!("http://{}/", url),
        }
    }
}

impl Client for TestClient {
    #[inline(always)]
    fn url(&self) -> &str {
        &self.url
    }
}

impl AppClient for TestClient {}
impl ChartClient for TestClient {}
impl UserClient for TestClient {}
impl LogClient for TestClient {}
