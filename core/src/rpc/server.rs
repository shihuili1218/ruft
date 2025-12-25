use crate::node::Node;
use crate::rpc::ruft_rpc_server::{RuftRpc, RuftRpcServer};
use crate::rpc::{
    AppendEntriesRequest, AppendEntriesResponse, PreVoteRequest, PreVoteResponse,
    RequestVoteRequest, RequestVoteResponse,
};
use tonic::{Request, Response, Status};

pub async fn start_server(server: Node) -> Result<(), Box<dyn std::error::Error>> {
    let addr = "127.0.0.1:1218".parse()?;
    tonic::transport::Server::builder()
        .add_service(RuftRpcServer::new(server))
        .serve(addr)
        .await?;
    Ok(())
}

#[tonic::async_trait]
impl RuftRpc for Node {
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
