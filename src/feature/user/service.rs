use super::interface::user_server::UserServer;

#[derive(Debug, Default)]
pub struct UserService;

pub fn build_service() -> UserServer<UserService> {
    let service = UserService::default();
    UserServer::new(service)
}
