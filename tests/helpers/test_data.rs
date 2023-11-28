use ratings::app::AppContext;
use sqlx::{pool::PoolConnection, Postgres};

use super::client_app::AppClient;
use super::client_user::UserClient;
use super::client_chart::ChartClient;

#[derive(Debug, Clone)]
pub struct TestData {
    pub user_client: Option<UserClient>,
    pub app_client: Option<AppClient>,
    pub chart_client: Option<ChartClient>,
    pub id: Option<String>,
    pub snap_id: Option<String>,
    pub token: Option<String>,
    pub app_ctx: AppContext,
}

impl TestData {
    pub async fn repository(&self) -> Result<PoolConnection<Postgres>, sqlx::Error> {
        self.app_ctx.clone().infrastructure().repository().await
    }

    pub fn socket(&self) -> String {
        self.app_ctx.config().socket()
    }
}
