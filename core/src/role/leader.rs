use crate::role::state::{Role, Common};
use crate::role::Follower;
use crate::rpc::Endpoint;
use std::collections::HashMap;
use std::fmt::Display;
use std::sync::Arc;

/// Leader state: managing replication to followers
#[derive(Clone)]
pub struct Leader {
    pub term: u64,
    pub next_index: HashMap<Endpoint, u64>,
    pub match_index: HashMap<Endpoint, u64>,
    pub common: Arc<Common>,
}

impl Role for Leader {
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

/// Business logic for Leader role
impl Leader {
    /// Prepare heartbeat requests for all followers
    pub async fn prepare_heartbeat_requests(&self) -> Vec<(Endpoint, HeartbeatRequest)> {
        let meta = self.common.meta.lock().await;
        let mut requests = Vec::new();

        for (endpoint, &next_idx) in &self.next_index {
            let prev_log_index = if next_idx > 0 { next_idx - 1 } else { 0 };
            let prev_log_term = 0; // TODO: Get from log

            requests.push((
                endpoint.clone(),
                HeartbeatRequest {
                    term: self.term,
                    leader_id: self.common.endpoint.id(),
                    prev_log_index,
                    prev_log_term,
                    leader_commit: meta.committed_index(),
                },
            ));
        }

        requests
    }

    /// Handle AppendEntries response from follower
    pub fn handle_append_response(
        &mut self,
        follower: Endpoint,
        success: bool,
        match_index: u64,
    ) {
        if success {
            self.match_index.insert(follower.clone(), match_index);
            self.next_index.insert(follower, match_index + 1);
            // TODO: Update commit index if majority replicated
        } else {
            // Decrement next_index and retry
            if let Some(next_idx) = self.next_index.get_mut(&follower) {
                if *next_idx > 0 {
                    *next_idx -= 1;
                }
            }
        }
    }

    /// Discovered higher term - step down to Follower
    pub fn step_down(self, new_term: u64, leader: Endpoint) -> Follower {
        Follower {
            term: new_term,
            leader,
            common: self.common,
        }
    }
}

/// Heartbeat request (empty AppendEntries)
#[derive(Debug, Clone)]
pub struct HeartbeatRequest {
    pub term: u64,
    pub leader_id: u8,
    pub prev_log_index: u64,
    pub prev_log_term: u64,
    pub leader_commit: u64,
}
