use std::str::FromStr;

use axum::async_trait;
use log::Level;
use ratings::features::admin::log_level::interface::{GetLogLevelResponse, SetLogLevelRequest};
use reqwest::Url;

use super::Client;

#[async_trait]
pub trait LogClient: Client {
    fn rest_url(&self) -> Url {
        Url::from_str(self.url())
            .unwrap()
            .join("/v1/admin/log-level")
            .unwrap()
    }

    async fn get_log_level(
        &self,
    ) -> Result<GetLogLevelResponse, Box<dyn std::error::Error + Send + Sync>> {
        Ok(serde_json::from_str(
            &reqwest::get(self.rest_url())
                .await?
                .error_for_status()?
                .text()
                .await?,
        )?)
    }

    async fn set_log_level(
        &self,
        level: Level,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        reqwest::Client::new()
            .post(self.rest_url())
            .header("Content-Type", "application/json")
            .body(serde_json::to_string(&SetLogLevelRequest { level }).unwrap())
            .send()
            .await?
            .error_for_status_ref()?;

        Ok(())
    }
}
