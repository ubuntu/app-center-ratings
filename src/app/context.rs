use crate::utils::jwt::Claims;

#[derive(Debug)]
pub struct Context {
    pub uri: String,
    pub claims: Option<Claims>
}
