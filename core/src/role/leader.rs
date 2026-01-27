use crate::role::state::{Common, Role};
use crate::role::Follower;
use crate::rpc::Endpoint;
use crate::storage::LogStore;
use std::collections::HashMap;
use std::fmt::Display;
use std::sync::Arc;

pub struct Replication {
    logs: Arc<LogStore>,
    confirmed: u64,   // 等价于 nextIndex - 1
    match_index: u64, // 可选：用于 commit 计算
    inflight: Vec<u64>,
}

/// Leader state: managing replication to followers
#[derive(Clone)]
pub struct Leader {
    my_id: u8,
    term: u64,
    preparing: bool,
    next_index: HashMap<u8, u64>,
    match_index: HashMap<u8, u64>,
    common: Arc<Common>,
}

impl Leader {
    pub fn new(my_id: u8, term: u64, match_index: HashMap<u8, u64>, common: Arc<Common>) -> Self {
        Self {
            my_id,
            term,
            preparing: false,
            next_index: HashMap::new(),
            match_index,
            common,
        }
    }

    pub fn term(&self) -> u64 {
        self.term
    }
}

impl Role for Leader {
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

impl Display for Leader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Leader[term={}, followers={}]", self.term, self.next_index.len())
    }
}

/// Business logic for Leader role
impl Leader {
    pub async fn become_leader(&mut self) {
        self.send_append_entries().await;

        // todo: probe msg: heartbeat?
        // todo: merge log entry
        self.preparing = true;
    }

    pub async fn send_append_entries(&self) {
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

        todo!()
    }

    /// Handle AppendEntries response from follower
    pub fn handle_append_response(&mut self, follower: Endpoint, success: bool, match_index: u64) {
        if success {
            self.match_index.insert(follower.id(), match_index);
            self.next_index.insert(follower.id(), match_index + 1);
            // TODO: Update commit index if majority replicated
        } else {
            // Decrement next_index and retry
            if let Some(next_idx) = self.next_index.get_mut(&follower.id()) {
                if *next_idx > 0 {
                    *next_idx -= 1;
                }
            }
        }
    }

    /// Discovered higher term - step down to Follower
    pub fn transition_follower(self, new_term: u64, leader: Endpoint) -> Follower {
        Follower::new(self.my_id, new_term, leader, self.common)
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
