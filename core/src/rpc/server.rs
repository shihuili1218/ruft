use crate::node::node::Node;
use crate::rpc::ruft_rpc_server::{RuftRpc, RuftRpcServer};
use crate::rpc::{AppendEntriesRequest, AppendEntriesResponse, PreVoteRequest, PreVoteResponse, RequestVoteRequest, RequestVoteResponse};
use std::error::Error;
use std::sync::Arc;
use tonic::{Request, Response, Status};
use tracing::info;

/// RPC server adapter
/// Handles protocol conversion between gRPC and domain layer
pub struct RuftServer {
    node: Arc<Node>,
}

impl RuftServer {
    pub fn new(node: Arc<Node>) -> Self {
        Self { node }
    }

    pub async fn start(self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let addr = "127.0.0.1:1218".parse()?;

        info!("Rpc server is starting");
        tonic::transport::Server::builder()
            .add_service(RuftRpcServer::new(self))
            .serve(addr)
            .await?;
        info!("Rpc server is started");
        Ok(())
    }
}

#[tonic::async_trait]
impl RuftRpc for RuftServer {
    async fn pre_vote(&self, request: Request<PreVoteRequest>) -> Result<Response<PreVoteResponse>, Status> {
        let req = request.into_inner();

        let (vote_granted, term) = self.node
            .pre_vote(req.candidate_id, req.term, req.last_log_index, req.last_log_term)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        // Protocol conversion: build response
        Ok(Response::new(PreVoteResponse {
            term,
            vote_granted,
        }))
    }

    async fn request_vote(&self, request: Request<RequestVoteRequest>) -> Result<Response<RequestVoteResponse>, Status> {
        let req = request.into_inner();

        // Call domain layer
        let (vote_granted, term) = self.node
            .vote(req.candidate_id, req.term, req.last_log_index, req.last_log_term)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        // Protocol conversion: build response
        Ok(Response::new(RequestVoteResponse {
            term,
            vote_granted,
        }))
    }

    async fn append_entries(&self, request: Request<AppendEntriesRequest>) -> Result<Response<AppendEntriesResponse>, Status> {
        let req = request.into_inner();

        // Call domain layer (entries conversion TODO)
        let (success, _match_index, term) = self.node
            .append_entries(
                req.leader_id,
                req.term,
                req.prev_log_index,
                req.prev_log_term,
                vec![], // TODO: Convert req.entries
                req.leader_commit,
            )
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        // Protocol conversion: build response
        Ok(Response::new(AppendEntriesResponse {
            term,
            success,
        }))
    }
}
