use crate::Result;
use crate::RuftError;
use crate::rpc::ruft_rpc_client::RuftRpcClient;
use crate::rpc::{Endpoint, PreVoteRequest, PreVoteResponse, RequestVoteRequest, RequestVoteResponse};
use dashmap::DashMap;
use std::sync::Arc;
use tonic::transport::Channel;
use tonic::transport::Endpoint as TonicEndpoint;
use tracing::error;

pub async fn init_rpc_clients(endpoints: Vec<Endpoint>) -> Vec<RemoteClient> {
    let futures: Vec<_> = endpoints
        .iter()
        .map(|endpoint| {
            let endpoint = endpoint.clone();
            async move { init_remote_client(&endpoint).await.inspect_err(|e| error!("Failed to init remote client: {}", e)) }
        })
        .collect();

    futures::future::join_all(futures).await.into_iter().filter_map(Result::ok).collect()
}

async fn init_remote_client(endpoint: &Endpoint) -> Result<RemoteClient> {
    let channel = TonicEndpoint::from_shared(endpoint.url())
        .map_err(|_| RuftError::Configuration(format!("failed to parse Tonic channel: {}", endpoint.url()).into()))?
        .connect()
        .await
        .map_err(|e| RuftError::Network(format!("failed to connect to Tonic channel: {}", e).into()))?;
    let client = RuftRpcClient::new(channel);
    Ok(RemoteClient {
        my_id: endpoint.id(),
        is_voting: endpoint.is_voting(),
        client,
    })
}

pub trait RaftRpcClient {
    async fn close(&self) -> Result<()>;
    async fn pre_vote(&mut self, term: u64, candidate_id: u8, last_log_id: u64, last_log_term: u64) -> Result<PreVoteResponse>;
    async fn request_vote(&mut self, term: u64, candidate_id: u8, last_log_id: u64, last_log_term: u64) -> Result<RequestVoteResponse>;
}

#[derive(Clone)]
pub struct RemoteClient {
    my_id: u8,
    is_voting: bool,
    client: RuftRpcClient<Channel>,
}

impl RemoteClient {
    pub fn my_id(&self) -> u8 {
        self.my_id
    }
    pub fn is_voter(&self) -> bool {
        self.is_voting
    }
}

impl RaftRpcClient for RemoteClient {
    async fn close(&self) -> Result<()> {
        Ok(())
    }

    async fn pre_vote(&mut self, term: u64, candidate_id: u8, last_log_id: u64, last_log_term: u64) -> Result<PreVoteResponse> {
        let request = PreVoteRequest {
            term,
            candidate_id: candidate_id.try_into().map_err(|_| RuftError::Configuration("candidate_id out of range".into()))?,
            last_log_index: last_log_id,
            last_log_term,
        };
        self.client
            .pre_vote(request)
            .await
            .map_err(|e| RuftError::Network(format!("pre_vote failed: {}", e)))
            .map(|resp| resp.into_inner())
    }

    async fn request_vote(&mut self, term: u64, candidate_id: u8, last_log_id: u64, last_log_term: u64) -> Result<RequestVoteResponse> {
        let request = RequestVoteRequest {
            term,
            candidate_id: candidate_id.try_into().map_err(|_| RuftError::Configuration("candidate_id out of range".into()))?,
            last_log_index: last_log_id,
            last_log_term,
        };
        self.client
            .request_vote(request)
            .await
            .map_err(|e| RuftError::Network(format!("request_vote failed: {}", e)))
            .map(|resp| resp.into_inner())
    }
}
