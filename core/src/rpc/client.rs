use crate::rpc::ruft_rpc_client::RuftRpcClient;
use crate::rpc::{Endpoint, PreVoteRequest, PreVoteResponse};
use std::error::Error;
use tonic::transport::Channel;
use tonic::transport::Endpoint as TonicEndpoint;

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
