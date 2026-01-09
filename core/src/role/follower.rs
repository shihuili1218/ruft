use crate::role::state::RaftState;
use crate::rpc::Endpoint;
use std::fmt::Display;

/// Follower state: waiting for heartbeats from leader
#[derive(Debug, Clone)]
pub struct Follower {
    pub term: u64,
    pub leader: Endpoint,
    pub voted_for: Option<u64>,
}

impl RaftState for Follower {
    fn term(&self) -> u64 {
        self.term
    }

    fn state_name() -> &'static str {
        "Follower"
    }
}

impl Display for Follower {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Following[term={}, leader={}]", self.term, self.leader)
    }
}
