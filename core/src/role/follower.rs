use crate::role::Candidate;
use crate::role::state::{Common, Role};
use crate::rpc::Endpoint;
use std::fmt::Display;
use std::sync::Arc;

/// Follower state: waiting for heartbeats from leader
#[derive(Clone)]
pub struct Follower {
    pub my_id: u8,
    pub term: u64,
    pub leader: Endpoint,
    pub common: Arc<Common>,
}

impl Role for Follower {}

impl Display for Follower {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Follower[term={}, leader={}]", self.term, self.leader)
    }
}

/// Business logic for Follower role
impl Follower {
    /// Handle AppendEntries RPC from leader
    pub async fn handle_append_entries(&mut self, leader_term: u64, leader: Endpoint, _prev_log_index: u64, _prev_log_term: u64, _entries: Vec<()>, _leader_commit: u64) -> crate::Result<(bool, u64)> {
        // Update term and leader if necessary
        if leader_term >= self.term {
            self.term = leader_term;
            self.leader = leader;
        }

        // TODO: Implement log replication logic
        Ok((true, 0))
    }

    pub async fn transition_candidate(self) -> Candidate {
        Candidate {
            my_id: self.my_id,
            pre_vote_term: self.term,
            common: self.common,
        }
    }
}
