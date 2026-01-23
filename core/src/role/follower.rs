use crate::role::state::{Role, Common};
use crate::role::Candidate;
use crate::rpc::Endpoint;
use std::fmt::Display;
use std::sync::Arc;

/// Follower state: waiting for heartbeats from leader
#[derive(Clone)]
pub struct Follower {
    pub term: u64,
    pub leader: Endpoint,
    pub common: Arc<Common>,
}

impl Role for Follower {
    fn term(&self) -> u64 {
        self.term
    }

    fn state_name() -> &'static str {
        "Follower"
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
    pub async fn handle_append_entries(
        &mut self,
        leader_term: u64,
        leader: Endpoint,
        _prev_log_index: u64,
        _prev_log_term: u64,
        _entries: Vec<()>,
        _leader_commit: u64,
    ) -> crate::Result<(bool, u64)> {
        // Update term and leader if necessary
        if leader_term >= self.term {
            self.term = leader_term;
            self.leader = leader;
        }

        // TODO: Implement log replication logic
        Ok((true, 0))
    }

    /// Election timeout - transition to Candidate
    pub async fn start_election(self) -> crate::Result<Candidate> {
        let my_id = self.common.endpoint.id();
        let new_term = {
            let mut meta = self.common.meta.lock().await;
            let term = meta.next_term()?;
            meta.set_voted_for(term, my_id)?;
            term
        };

        Ok(Candidate {
            id: my_id as u64,
            term: new_term,
            votes_received: 1,
            voted_for: my_id,
            common: self.common,
        })
    }
}
