pub mod follower {
    use crate::node::node::Node;
    use crate::rpc::{AppendEntriesRequest, AppendEntriesResponse, PreVoteRequest, PreVoteResponse, RequestVoteRequest, RequestVoteResponse};

    pub fn on_pre_vote(node: &Node, request: PreVoteRequest) -> PreVoteResponse {
        todo!()
    }

    pub fn on_vote(node: &Node, request: RequestVoteRequest) -> RequestVoteResponse {
        todo!()
    }

    pub fn on_append_entry(node: &Node, request: AppendEntriesRequest) -> AppendEntriesResponse {
        todo!()
    }
}
