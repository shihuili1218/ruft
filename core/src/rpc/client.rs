use std::sync::Arc;
use dashmap::DashMap;
use crate::rpc::ruft_rpc_client::RuftRpcClient;
use crate::rpc::{Endpoint, PreVoteRequest, PreVoteResponse, RequestVoteResponse};
use crate::Result;
use crate::RuftError;
use tonic::transport::Channel;
use tonic::transport::Endpoint as TonicEndpoint;
use tracing::error;

pub async fn init_rpc_clients(endpoints: Vec<Endpoint>) -> (DashMap<u8, RemoteClient>, DashMap<u8, RemoteClient>) {
    let futures: Vec<_> = endpoints
        .iter()
        .map(|endpoint| {
            let endpoint = endpoint.clone();
            async move {
                init_remote_client(&endpoint)
                    .await
                    .map(|client| (endpoint, client))
            }
        })
        .collect();

    let results = futures::future::join_all(futures).await;

    let votes = DashMap::new();
    let non_votes = DashMap::new();

    for result in results {
        match result {
            Ok((endpoint, client)) => {
                if endpoint.is_voting() {
                    votes.insert(endpoint.id(), client);
                } else {
                    non_votes.insert(endpoint.id(), client);
                }
            }
            Err(e) => {
                error!("Failed to init remote client: {}", e);
            }
        }
    }

    (votes, non_votes)
}

async fn init_remote_client(endpoint: &Endpoint) -> Result<RemoteClient> {
    let channel = TonicEndpoint::from_shared(endpoint.url())
        .map_err(|_| RuftError::Configuration(format!("failed to parse Tonic channel: {}", endpoint.url()).into()))?
        .connect()
        .await
        .map_err(|e| RuftError::Network(format!("failed to connect to Tonic channel: {}", e).into()))?;
    let client = RuftRpcClient::new(channel);
    Ok(RemoteClient { my_id: endpoint.id(), client })
}

pub trait RaftRpcClient {
    async fn close(&self) -> Result<()>;
    async fn pre_vote(&mut self, term: u64, candidate_id: u8, last_log_id: u64, last_log_term: u64) -> Result<PreVoteResponse>;
    async fn request_vote(&mut self, term: u64, candidate_id: u8, last_log_id: u64, last_log_term: u64) -> Result<RequestVoteResponse>;
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
        todo!()
    }
}
