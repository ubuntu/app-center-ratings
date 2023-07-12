mod after_test;
pub mod assert;
mod before_test;
pub mod infra;

pub mod lifecycle {
    pub use super::after_test::after;
    pub use super::before_test::before;
}

pub fn server_url() -> String {
    format!("http://localhost:18080")
}
