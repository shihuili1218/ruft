use crate::endpoint::Endpoint;
use std::collections::HashMap;
use std::fmt::Display;

/// Marker trait for valid Raft node states.
/// This enables the typestate pattern: RaftNode<S: RaftState>
pub trait RaftState: Sized {
    fn term(&self) -> u64;
    fn state_name() -> &'static str;
}

/// Follower state: waiting for heartbeats from leader
#[derive(Debug, Clone)]
pub struct Follower {
    pub term: u64,
    pub leader: Endpoint,
    pub voted_for: Option<u64>,
}

impl RaftState for Follower {
    fn term(&self) -> u64 {
        self.term
    }

    fn state_name() -> &'static str {
        "Follower"
    }
}

impl Display for Follower {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Following[term={}, leader={}]", self.term, self.leader)
    }
}

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

/// Leader state: managing replication to followers
/// Only the Leader has next_index and match_index - type system enforces this!
#[derive(Debug, Clone)]
pub struct Leader {
    pub term: u64,
    /// For each server, index of the next log entry to send
    pub next_index: HashMap<Endpoint, u64>,
    /// For each server, index of highest log entry known to be replicated
    pub match_index: HashMap<Endpoint, u64>,
}

impl RaftState for Leader {
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

/// Learner state: non-voting member that only receives log replication
#[derive(Debug, Clone)]
pub struct Learner {
    pub term: u64,
    pub leader: Endpoint,
}

impl RaftState for Learner {
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
