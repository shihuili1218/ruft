use crate::role::state::RaftState;
use std::fmt::Display;

/// Candidate state: requesting votes to become leader
#[derive(Debug, Clone)]
pub struct Candidate {
    pub term: u64,
    pub votes_received: u64,
    pub voted_for: u8,
}

impl RaftState for Candidate {
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
