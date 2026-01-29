pub(crate) mod client;
pub mod command;
mod endpoint;
pub(crate) mod server;

pub use crate::rpc::endpoint::Endpoint;

tonic::include_proto!("ruft");
