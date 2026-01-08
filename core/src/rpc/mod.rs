// Fix name conflict between our crate `core` and std's `core`
// Disable doctests for this module due to protobuf codegen conflicts
#![cfg_attr(doctest, allow(unused))]

#[allow(unused_extern_crates)]
extern crate std;

pub mod command;
mod endpoint;

use crate::node::node::Node;
pub use crate::rpc::endpoint::Endpoint;
use crate::rpc::ruft_rpc_client::RuftRpcClient;
use crate::rpc::ruft_rpc_server::{RuftRpc, RuftRpcServer};
use std::error::Error;
use std::future;
use std::marker;
use std::sync::Arc;
use tonic::transport::Channel;
use tonic::transport::Endpoint as TonicEndpoint;
use tonic::{Request, Response, Status};
use tracing::info;

tonic::include_proto!("ruft");

pub async fn run_server(node: Arc<Node>) -> Result<(), Box<dyn Error + Send + Sync>> {
    let addr = "127.0.0.1:1218".parse()?;
    info!("Rpc server is starting");
    tonic::transport::Server::builder().add_service(RuftRpcServer::from_arc(node)).serve(addr).await?;
    info!("Rpc server is started");
    Ok(())
}

#[tonic::async_trait]
impl RuftRpc for Node {
    async fn pre_vote(&self, request: Request<PreVoteRequest>) -> Result<Response<PreVoteResponse>, Status> {
        todo!()
    }

    async fn request_vote(&self, request: Request<RequestVoteRequest>) -> Result<Response<RequestVoteResponse>, Status> {
        todo!()
    }

    async fn append_entries(&self, request: Request<AppendEntriesRequest>) -> Result<Response<AppendEntriesResponse>, Status> {
        todo!()
    }
}

// ========================= client =========================

pub async fn init_remote_client(endpoint: &Endpoint) -> Result<RemoteClient, Box<dyn Error + Send + Sync>> {
    let channel = TonicEndpoint::from_shared(endpoint.url())?.connect().await?;
    let client = RuftRpcClient::new(channel);
    Ok(RemoteClient { client })
}

pub trait RaftRpcClient {
    async fn close(&self) -> Result<(), Box<dyn Error>>;
    async fn pre_vote(&mut self, term: u64, candidate_id: u64, last_log_id: u64, last_log_term: u64) -> Result<PreVoteResponse, Box<dyn std::error::Error>>;
}

pub struct RemoteClient {
    client: RuftRpcClient<Channel>,
}

impl RaftRpcClient for RemoteClient {
    async fn close(&self) -> Result<(), Box<dyn Error>> {
        Ok(())
    }

    async fn pre_vote(&mut self, term: u64, candidate_id: u64, last_log_id: u64, last_log_term: u64) -> Result<PreVoteResponse, Box<dyn std::error::Error>> {
        let resp = self
            .client
            .pre_vote(PreVoteRequest {
                term,
                candidate_id,
                last_log_index: last_log_id,
                last_log_term,
            })
            .await?;
        Ok(resp.into_inner())
    }
}
