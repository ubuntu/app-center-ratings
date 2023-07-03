use super::interface::protobuf::RegisterServer;

#[derive(Debug, Default)]
pub struct RegisterService;


pub fn build_service() -> RegisterServer<RegisterService> {
    let service = RegisterService::default();
    RegisterServer::new(service)
}
