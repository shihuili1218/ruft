use crate::rpc::RequestVoteRequest;
use crate::rpc::ruft_rpc_client::RuftRpcClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = RuftRpcClient::connect("http://127.0.0.1:50051").await?;

    let resp = client
        .request_vote(RequestVoteRequest {
            term: 1,
            candidate_id: 2,
            last_log_index: 10,
            last_log_term: 1,
        })
        .await?;

    println!("vote granted = {}", resp.into_inner().vote_granted);

    Ok(())
}
