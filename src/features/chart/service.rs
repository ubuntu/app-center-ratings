use crate::features::pb::chart::chart_server::ChartServer;

#[derive(Debug, Default)]
pub struct ChartService;

pub fn build_service() -> ChartServer<ChartService> {
    let service = ChartService;
    ChartServer::new(service)
}
