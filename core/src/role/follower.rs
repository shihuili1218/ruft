use crate::endpoint::Endpoint;
use crate::rpc::{AppendEntriesRequest, AppendEntriesResponse, PreVoteRequest, PreVoteResponse, RequestVoteRequest, RequestVoteResponse};

pub struct Follower {
    endpoint: Endpoint,
    pub leader: Endpoint,
}

impl Follower {
    pub fn new(endpoint: Endpoint, leader: Endpoint) -> Self {
        Follower { endpoint, leader }
    }

    pub fn on_pre_vote(&self, request: PreVoteRequest) -> PreVoteResponse {
        todo!()
    }

    pub fn on_vote(&self, request: RequestVoteRequest) -> RequestVoteResponse {
        todo!()
    }

    pub fn on_append_entry(&self, request: AppendEntriesRequest) -> AppendEntriesResponse {
        todo!()
    }
}
