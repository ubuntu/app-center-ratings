//! Application level context & state
use crate::{
    config::Config,
    jwt::{Error, JwtEncoder},
};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{Mutex, Notify};

pub struct Context {
    pub config: Config,
    pub jwt_encoder: JwtEncoder,
    pub http_client: reqwest::Client,

    /// In progress category updates that we need to block on
    pub category_updates: Mutex<HashMap<String, Arc<Notify>>>,
}

impl Context {
    pub fn new(config: Config) -> Result<Self, Error> {
        let jwt_encoder = JwtEncoder::from_secret(&config.jwt_secret)?;

        Ok(Self {
            config,
            jwt_encoder,
            http_client: reqwest::Client::new(),
            category_updates: Default::default(),
        })
    }
}
