use crate::role::state::{Role, Common};
use crate::rpc::Endpoint;
use std::fmt::Display;
use std::sync::Arc;

/// Learner state: non-voting member that only receives log replication
#[derive(Clone)]
pub struct Learner {
    pub term: u64,
    pub leader: Endpoint,
    pub common: Arc<Common>,
}

impl Role for Learner {
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
