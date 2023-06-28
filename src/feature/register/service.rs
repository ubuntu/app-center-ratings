use std::sync::Arc;

use crate::app::infrastructure::Infrastructure;

use super::interface::protobuf::RegisterServer;

#[derive(Debug)]
pub struct RegisterService {
    pub infra: Arc<Infrastructure>,
}

impl RegisterService {
    fn new(infra: Arc<Infrastructure>) -> Self {
        RegisterService { infra }
    }
}

pub fn build_service(infra: Arc<Infrastructure>) -> RegisterServer<RegisterService> {
    let service = RegisterService::new(infra);
    RegisterServer::new(service)
}
