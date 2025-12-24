use crate::role::follower::Follower;
use crate::role::learner::Learner;
use crate::rpc::ruft_rpc_server::{RuftRpc, RuftRpcServer};
use crate::rpc::{
    AppendEntriesRequest, AppendEntriesResponse, PreVoteRequest, PreVoteResponse,
    RequestVoteRequest, RequestVoteResponse,
};
use tonic::{Request, Response, Status};

#[tokio::main]
async fn start_server(
    follower_rpc: FollowerRpcServer,
    learner_rpc: LearnerRpcServer,
) -> Result<(), Box<dyn std::error::Error>> {
    let addr = "127.0.0.1:1218".parse()?;

    tonic::transport::Server::builder()
        .add_service(RuftRpcServer::new(follower_rpc))
        .add_service(RuftRpcServer::new(learner_rpc))
        .serve(addr)
        .await?;
    Ok(())
}

struct FollowerRpcServer {
    follower: Follower,
}
struct LearnerRpcServer {
    learner: Learner,
}

#[tonic::async_trait]
impl RuftRpc for LearnerRpcServer {
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
#[tonic::async_trait]
impl RuftRpc for FollowerRpcServer {
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

        todo!();
        // let granted = req.term >= self.current_term;
        // Ok(Response::new(RequestVoteResponse {
        //     term: self.current_term,
        //     vote_granted: granted,
        // }))
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

        todo!();
        // Ok(Response::new(AppendEntriesResponse {
        //     term: self.current_term,
        //     success: true,
        // }))
    }
}
