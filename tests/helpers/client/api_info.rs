use std::str::FromStr;

use axum::async_trait;
use ratings::{
    app::interfaces::authentication::admin::AdminAuthConfig,
    features::admin::api_version::interface::ApiVersionResponse,
};
use reqwest::Url;
use secrecy::ExposeSecret;

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
        let (un, pass) = AdminAuthConfig::from_env()
            .expect("could not decode admin secrets from env")
            .into_inner();

        let text_response = reqwest::Client::new()
            .get(self.rest_url())
            .basic_auth(un.expose_secret(), Some(pass.expose_secret()))
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;

        Ok(serde_json::from_str(&text_response)?)
    }
}
