use crate::role::state::{Role, Common};
use crate::role::{Follower, Leader};
use crate::rpc::Endpoint;
use std::fmt::Display;
use std::sync::Arc;

/// Candidate state: requesting votes to become leader
#[derive(Clone)]
pub struct Candidate {
    pub term: u64,
    pub votes_received: u64,
    pub voted_for: u8,
    pub common: Arc<Common>,
}

impl Role for Candidate {
    fn term(&self) -> u64 {
        self.term
    }

    fn state_name() -> &'static str {
        "Candidate"
    }
}

impl Display for Candidate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Candidate[term={}, votes={}]", self.term, self.votes_received)
    }
}

pub enum VoteResult {
    Won,
    Lost,
    InProgress,
}

/// Business logic for Candidate role
impl Candidate {
    /// Record a vote response
    pub fn handle_vote_response(&mut self, granted: bool, total_nodes: usize) -> VoteResult {
        if granted {
            self.votes_received += 1;
        }

        let majority = (total_nodes / 2) + 1;
        if self.votes_received >= majority as u64 {
            VoteResult::Won
        } else {
            VoteResult::InProgress
        }
    }

    /// Discovered a leader - step down to Follower
    pub fn step_down(self, leader_term: u64, leader: Endpoint) -> Follower {
        Follower {
            term: leader_term,
            leader,
            common: self.common,
        }
    }

    /// Won election - become Leader
    pub async fn become_leader(self) -> crate::Result<Leader> {
        use std::collections::HashMap;

        let (members, last_log_index) = {
            let meta = self.common.meta.lock().await;
            (meta.members(), meta.last_log_id())
        };

        let mut next_index = HashMap::new();
        let mut match_index = HashMap::new();

        for member in members {
            if member != self.common.endpoint {
                next_index.insert(member.clone(), last_log_index + 1);
                match_index.insert(member, 0);
            }
        }

        Ok(Leader {
            term: self.term,
            next_index,
            match_index,
            common: self.common,
        })
    }
}
