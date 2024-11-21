use crate::db;
use tonic::Status;

pub mod app;
pub mod charts;
pub mod user;

impl From<db::Error> for Status {
    fn from(value: db::Error) -> Self {
       Status::internal(value.to_string())
    }
}
