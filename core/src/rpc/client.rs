use crate::rpc::ruft_rpc_client::RuftRpcClient;
use crate::rpc::{Endpoint, PreVoteRequest, PreVoteResponse, RequestVoteRequest, RequestVoteResponse};
use crate::RuftError;
use std::error::Error;
use tonic::Status;
use tonic::transport::Channel;
use tonic::transport::Endpoint as TonicEndpoint;

pub async fn init_remote_client(endpoint: &Endpoint) -> Result<RemoteClient, Box<dyn Error + Send + Sync>> {
    let channel = TonicEndpoint::from_shared(endpoint.url())?.connect().await?;
    let client = RuftRpcClient::new(channel);
    Ok(RemoteClient { my_id: endpoint.id(), client })
}

pub trait RaftRpcClient {
    async fn close(&self) -> crate::Result<()>;
    async fn pre_vote(&mut self, term: u64, candidate_id: u8, last_log_id: u64, last_log_term: u64) -> crate::Result<PreVoteResponse>;
    async fn request_vote(&mut self, term: u64, candidate_id: u8, last_log_id: u64, last_log_term: u64) -> crate::Result<RequestVoteResponse>;
}

#[derive(Clone)]
pub struct RemoteClient {
    my_id: u8,
    client: RuftRpcClient<Channel>,
}

impl RemoteClient {
    pub fn my_id(&self) -> u8 {
        self.my_id
    }
}

impl RaftRpcClient for RemoteClient {
    async fn close(&self) -> crate::Result<()> {
        Ok(())
    }

    async fn pre_vote(&mut self, term: u64, candidate_id: u8, last_log_id: u64, last_log_term: u64) -> crate::Result<PreVoteResponse> {
        let request = PreVoteRequest {
            term,
            candidate_id: candidate_id.try_into()
                .map_err(|_| RuftError::Configuration("candidate_id out of range".into()))?,
            last_log_index: last_log_id,
            last_log_term,
        };
        self
            .client
            .pre_vote(request)
            .await
            .map_err(|e| RuftError::Network(format!("pre_vote failed: {}", e)))
            .map(|resp| resp.into_inner())
    }

    async fn request_vote(&mut self, term: u64, candidate_id: u8, last_log_id: u64, last_log_term: u64) -> crate::Result<RequestVoteResponse> {
        todo!()
    }
}
