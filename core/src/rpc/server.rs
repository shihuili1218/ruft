use crate::node::ruft::Ruft;
use crate::rpc::ruft_rpc_server::{RuftRpc, RuftRpcServer};
use crate::rpc::{
    AppendEntriesRequest, AppendEntriesResponse, PreVoteRequest, PreVoteResponse,
    RequestVoteRequest, RequestVoteResponse,
};
use std::sync::Arc;
use tonic::{Request, Response, Status};

pub async fn run_server(node: Arc<Ruft>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = "127.0.0.1:1218".parse()?;
    tonic::transport::Server::builder()
        .add_service(RuftRpcServer::new(node))
        .serve(addr)
        .await?;
    Ok(())
}

#[tonic::async_trait]
impl RuftRpc for Arc<Ruft> {
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
