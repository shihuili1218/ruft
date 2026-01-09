use crate::role::state::RaftState;
use crate::rpc::Endpoint;
use std::collections::HashMap;
use std::fmt::Display;

/// Leader state: managing replication to followers
/// Only the Leader has next_index and match_index - type system enforces this!
#[derive(Debug, Clone)]
pub struct Leader {
    pub term: u64,
    /// For each server, index of the next log entry to send
    pub next_index: HashMap<Endpoint, u64>,
    /// For each server, index of highest log entry known to be replicated
    pub match_index: HashMap<Endpoint, u64>,
}

impl RaftState for Leader {
    fn term(&self) -> u64 {
        self.term
    }

    fn state_name() -> &'static str {
        "Leader"
    }
}

impl Display for Leader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Leader[term={}, followers={}]", self.term, self.next_index.len())
    }
}
