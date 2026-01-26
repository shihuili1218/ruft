use crate::role::state::{Common, Role};
use crate::role::Candidate;
use crate::rpc::Endpoint;
use std::fmt::Display;
use std::sync::Arc;
use crate::Result;

/// Follower state: waiting for heartbeats from leader
#[derive(Clone)]
pub struct Follower {
    my_id: u8,
    term: u64,
    leader: Endpoint,
    common: Arc<Common>,
}

impl Follower {
    pub fn new(my_id: u8, term: u64, leader: Endpoint, common: Arc<Common>) -> Self {
        Self { my_id, term, leader, common }
    }
    pub fn term(&self) -> u64 {
        self.term
    }
    pub fn leader(&self) -> Endpoint {
        self.leader.clone()
    }
}

impl Role for Follower {
    fn my_id(&self) -> u8 {
        self.my_id
    }

    fn is_voter(&self) -> bool {
        true
    }

    fn common(&self) -> Arc<Common> {
        self.common.clone()
    }
}

impl Display for Follower {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Follower[term={}, leader={}]", self.term, self.leader)
    }
}

/// Business logic for Follower role
impl Follower {
    /// Handle AppendEntries RPC from leader
    pub async fn handle_append_entries(&mut self, leader_id: u8, leader_term: u64, prev_log_index: u64, prev_log_term: u64, entries: Vec<()>, leader_commit: u64) -> Result<AppendEntriesResult> {
        let meta = self.common.meta.lock().await;
        let current_term = meta.term();

        // Reject stale term
        if leader_term < current_term {
            return Ok(AppendEntriesResult {
                success: false,
                match_idx: 0,
                term: current_term,
            });
        }


        // Update term and leader if necessary
        if leader_term >= self.term {
            self.term = leader_term;
            // self.leader = leader;
        }

        // TODO: Implement log replication logic
        Ok(AppendEntriesResult {
            success: true,
            match_idx: 0,
            term: current_term,
        })
    }

    pub async fn transition_candidate(self) -> Candidate {
        Candidate::new(self.my_id, self.term, self.common)
    }
}

struct AppendEntriesResult {
    success: bool,
    match_idx: u64,
    term: u64,
}
