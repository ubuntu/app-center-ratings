use crate::features::pb::user::user_server::UserServer;

#[derive(Debug, Default)]
pub struct UserService;

pub fn build_service() -> UserServer<UserService> {
    let service = UserService;
    UserServer::new(service)
}
