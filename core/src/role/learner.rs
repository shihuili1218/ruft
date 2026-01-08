use crate::role::state::RaftState;
use crate::rpc::Endpoint;
use std::fmt::Display;

/// Learner state: non-voting member that only receives log replication
#[derive(Debug, Clone)]
pub struct Learner {
    pub term: u64,
    pub leader: Endpoint,
}

impl RaftState for Learner {
    fn term(&self) -> u64 {
        self.term
    }

    fn state_name() -> &'static str {
        "Learner"
    }
}

impl Display for Learner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Learner[term={}, leader={}]", self.term, self.leader)
    }
}
