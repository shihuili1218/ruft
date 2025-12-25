use crate::node::Node;
use crate::rpc::ruft_rpc_server::{RuftRpc, RuftRpcServer};
use crate::rpc::{
    AppendEntriesRequest, AppendEntriesResponse, PreVoteRequest, PreVoteResponse,
    RequestVoteRequest, RequestVoteResponse,
};
use std::sync::Arc;
use tonic::{Request, Response, Status};
use tracing::info;

pub async fn run_server(node: &Arc<Node>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let  node = node.clone();
    let addr = "127.0.0.1:1218".parse()?;
    info!("Rpc server is starting");
    tonic::transport::Server::builder()
        .add_service(RuftRpcServer::new(node))
        .serve(addr)
        .await?;
    info!("Rpc server is started");
    Ok(())
}

#[tonic::async_trait]
impl RuftRpc for Arc<Node> {
    async fn pre_vote(
        &self,
        request: Request<PreVoteRequest>,
    ) -> Result<Response<PreVoteResponse>, Status> {
        todo!()
    }

    async fn request_vote(
        &self,
        request: Request<RequestVoteRequest>,
    ) -> Result<Response<RequestVoteResponse>, Status> {
        todo!()
    }

    async fn append_entries(
        &self,
        request: Request<AppendEntriesRequest>,
    ) -> Result<Response<AppendEntriesResponse>, Status> {
        todo!()
    }
}
