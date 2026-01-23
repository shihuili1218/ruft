use crate::role::state::{Common, Role};
use crate::role::{Follower, Leader};
use crate::rpc::client::RaftRpcClient;
use crate::rpc::Endpoint;
use std::cmp::PartialEq;
use std::fmt::Display;
use std::result;
use std::sync::atomic::{AtomicU8, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tracing::error;

/// Candidate state: requesting votes to become leader
#[derive(Clone)]
pub struct Candidate {
    pub id: u64,
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

impl PartialEq for VoteResult {
    fn eq(&self, other: &Self) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}

/// Business logic for Candidate role
impl Candidate {
    async fn pre_vote(&mut self) -> crate::Result<VoteResult> {
        let total_nodes = self.common.voting_clients.len();
        let majority = (total_nodes / 2) + 1;
        let need_grant = AtomicUsize::new(majority);
        let refuse_grant = AtomicUsize::new(majority);
        need_grant.fetch_sub(1, Ordering::Relaxed);

        let (last_log_term, last_log_id) = {
            let guard = self.common.meta.lock().await;
            (guard.last_log_term(), guard.last_log_id())
        };

        let futures: Vec<_> = self
            .common
            .voting_clients
            .iter()
            .map(|entry| {
                let mut client = entry.value().clone();
                let term = self.term;
                let id = self.id;
                async move { client.pre_vote(term, id, last_log_id, last_log_term).await }
            })
            .collect();

        let rpc_timeout = Duration::from_millis(self.common.config.rpc_timeout_millis);

        match tokio::time::timeout(rpc_timeout, futures::future::join_all(futures)).await {
            Ok(results) => {
                for result in results {
                    match result {
                        Ok(response) => {
                            if response.vote_granted {
                                need_grant.fetch_sub(1, Ordering::Relaxed);
                                if need_grant.load(Ordering::Relaxed) == 0 {
                                    return Ok(VoteResult::Won);
                                }
                            } else {
                                refuse_grant.fetch_sub(1, Ordering::Relaxed);
                                if refuse_grant.load(Ordering::Relaxed) == 0 {
                                    return Ok(VoteResult::Lost);
                                }
                            }
                        }
                        Err(e) => {
                            error!("Candidate request vote failed: {}", e);
                        }
                    }
                }
            }
            Err(_) => {
                error!("Request vote timeout after {:?}", rpc_timeout);
            }
        }

        Ok(VoteResult::InProgress)
    }

    pub async fn request_vote(&mut self) -> crate::Result<VoteResult> {
        let total_nodes = self.common.voting_clients.len();
        let (last_log_term, last_log_id) = {
            let guard = self.common.meta.lock().await;
            (guard.last_log_term(), guard.last_log_id())
        };

        let futures: Vec<_> = self
            .common
            .voting_clients
            .iter()
            .map(|entry| {
                let mut client = entry.value().clone();
                let term = self.term;
                let id = self.id;
                async move { client.pre_vote(term, id, last_log_id, last_log_term).await }
            })
            .collect();

        let rpc_timeout = Duration::from_millis(self.common.config.rpc_timeout_millis);

        match tokio::time::timeout(rpc_timeout, futures::future::join_all(futures)).await {
            Ok(results) => {
                for result in results {
                    match result {
                        Ok(response) => {
                            return Ok(self.handle_vote_response(response.vote_granted, total_nodes));
                        }
                        Err(e) => {
                            error!("Candidate request vote failed: {}", e);
                        }
                    }
                }
            }
            Err(_) => {
                error!("Request vote timeout after {:?}", rpc_timeout);
            }
        }

        Ok(VoteResult::InProgress)
    }

    /// Record a vote response
    fn handle_vote_response(&mut self, granted: bool, total_nodes: usize) -> VoteResult {
        if granted {
            self.votes_received += 1;
        }

        let majority = (total_nodes / 2) + 1;
        if self.votes_received >= majority as u64 { VoteResult::Won } else { VoteResult::InProgress }
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
