use std::sync::Arc;

use crate::utils::{jwt::Claims, Config, Infrastructure};

#[derive(Debug, Clone)]
pub struct AppContext(Arc<AppContextInner>);

#[allow(dead_code)]
impl AppContext {
    pub fn new(config: &Config, infra: Infrastructure) -> Self {
        let inner = AppContextInner {
            infra,
            config: config.clone(),
        };
        Self(Arc::new(inner))
    }

    pub fn infrastructure(&self) -> &Infrastructure {
        &self.0.infra
    }

    pub fn config(&self) -> &Config {
        &self.0.config
    }
}

#[allow(dead_code)]
#[derive(Debug)]
struct AppContextInner {
    infra: Infrastructure,
    config: Config,
}

#[derive(Debug, Clone)]
pub struct RequestContext {
    pub uri: String,
    pub claims: Option<Claims>,
}
