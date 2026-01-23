use crate::role::state::{Common, Role};
use crate::rpc::Endpoint;
use std::fmt::Display;
use std::sync::Arc;

/// Learner state: non-voting member that only receives log replication
#[derive(Clone)]
pub struct Learner {
    pub my_id: u8,
    pub term: u64,
    pub leader: Endpoint,
    pub common: Arc<Common>,
}

impl Role for Learner {}

impl Display for Learner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Learner[term={}, leader={}]", self.term, self.leader)
    }
}
