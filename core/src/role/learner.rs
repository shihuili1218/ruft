use crate::role::state::{Common, Role};
use crate::rpc::Endpoint;
use std::fmt::Display;
use std::sync::Arc;

/// Learner state: non-voting member that only receives log replication
pub struct Learner {
    my_id: u8,
    term: u64,
    leader: Endpoint,
    common: Arc<Common>,
}

impl Learner {
    pub fn new(my_id: u8, term: u64, leader: Endpoint, common: Arc<Common>) -> Self {
        Self { my_id, term, leader, common }
    }
}

impl Role for Learner {
    fn my_id(&self) -> u8 {
        self.my_id
    }

    fn is_voter(&self) -> bool {
        false
    }

    fn common(&self) -> Arc<Common> {
        self.common.clone()
    }
}

impl Display for Learner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Learner[term={}, leader={}]", self.term, self.leader)
    }
}
