use crate::endpoint::Endpoint;
use crate::rpc::ruft_rpc_client::RuftRpcClient;
use crate::rpc::{PreVoteRequest, PreVoteResponse};
use tonic::transport::Channel;
use tonic::transport::Endpoint as TonicEndpoint;

struct RpcClient {
    client: RuftRpcClient<Channel>,
}

impl RpcClient {
    async fn connect(endpoint: &Endpoint) -> Result<Self, Box<dyn std::error::Error>> {
        let channel = TonicEndpoint::from_shared(endpoint.url.clone())?
            .connect()
            .await?;
        let client = RuftRpcClient::new(channel);
        Ok(RpcClient { client })
    }

    async fn close(&self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    async fn pre_vote(
        &mut self,
        term: u64,
        candidate_id: u64,
        last_log_id: u64,
        last_log_term: u64,
    ) -> Result<PreVoteResponse, Box<dyn std::error::Error>> {
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
