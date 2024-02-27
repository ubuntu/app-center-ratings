//! API endpoint definitions for different entry methods

pub mod grpc;

#[allow(unused_imports)]
pub use grpc::{GrpcService, GrpcServiceBuilder};
