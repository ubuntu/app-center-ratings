use std::str::FromStr;

use axum::async_trait;
use ratings::features::admin::api_version::interface::ApiVersionResponse;
use reqwest::Url;

use super::Client;

#[async_trait]
pub trait ApiInfoClient: Client {
    fn rest_url(&self) -> Url {
        Url::from_str(self.url())
            .unwrap()
            .join("/v1/admin/api-version")
            .unwrap()
    }

    async fn get_api_info(
        &self,
    ) -> Result<ApiVersionResponse, Box<dyn std::error::Error + Send + Sync>> {
        Ok(serde_json::from_str(
            &reqwest::get(self.rest_url())
                .await?
                .error_for_status()?
                .text()
                .await?,
        )?)
    }
}
