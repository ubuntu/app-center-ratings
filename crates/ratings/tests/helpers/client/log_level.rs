use std::str::FromStr;

use axum::async_trait;
use log::Level;
use ratings::{
    app::interfaces::authentication::admin::AdminAuthConfig,
    features::admin::log_level::interface::{GetLogLevelResponse, SetLogLevelRequest},
};
use reqwest::Url;
use secrecy::ExposeSecret;

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
        let (un, pass) = AdminAuthConfig::from_env()
            .expect("could not decode admin secrets from env")
            .into_inner();

        let text_response = reqwest::Client::new()
            .get(self.rest_url())
            .header("Content-Type", "application/json")
            .basic_auth(un.expose_secret(), Some(pass.expose_secret()))
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;

        Ok(serde_json::from_str(&text_response)?)
    }

    async fn set_log_level(
        &self,
        level: Level,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let (un, pass) = AdminAuthConfig::from_env()
            .expect("could not decode admin secrets from env")
            .into_inner();
        reqwest::Client::new()
            .post(self.rest_url())
            .header("Content-Type", "application/json")
            .basic_auth(un.expose_secret(), Some(pass.expose_secret()))
            .body(serde_json::to_string(&SetLogLevelRequest { level }).unwrap())
            .send()
            .await?
            .error_for_status_ref()?;

        Ok(())
    }
}
