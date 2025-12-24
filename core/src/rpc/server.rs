use crate::rpc::ruft_rpc_server::{RuftRpc, RuftRpcServer};
use crate::rpc::{
    AppendEntriesRequest, AppendEntriesResponse, PreVoteRequest, PreVoteResponse,
    RequestVoteRequest, RequestVoteResponse,
};
use tonic::{Request, Response, Status};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "127.0.0.1:1218".parse()?;
    let raft = RuftNode::default();

    tonic::transport::Server::builder()
        .add_service(RuftRpcServer::new(raft))
        .serve(addr)
        .await?;

    Ok(())
}

#[derive(Default)]
struct RuftNode {
    current_term: u64,
    voted_for: Option<u64>,
}
#[tonic::async_trait]
impl RuftRpc for RuftNode {
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
        let req = request.into_inner();

        println!(
            "RequestVote from {} for term {}",
            req.candidate_id, req.term
        );

        let granted = req.term >= self.current_term;

        Ok(Response::new(RequestVoteResponse {
            term: self.current_term,
            vote_granted: granted,
        }))
    }

    async fn append_entries(
        &self,
        request: Request<AppendEntriesRequest>,
    ) -> Result<Response<AppendEntriesResponse>, Status> {
        let req = request.into_inner();

        println!(
            "AppendEntries from leader {}, entries={}",
            req.leader_id,
            req.entries.len()
        );

        Ok(Response::new(AppendEntriesResponse {
            term: self.current_term,
            success: true,
        }))
    }
}
