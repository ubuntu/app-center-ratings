use super::infrastructure::Infrastructure;

#[derive(Debug)]
pub struct Context {
    pub uri: String,
    pub infra: Infrastructure,
}
