use crate::features::pb::app::app_server::AppServer;

#[derive(Debug, Default)]
pub struct AppService;

pub fn build_service() -> AppServer<AppService> {
    let service = AppService;
    AppServer::new(service)
}
