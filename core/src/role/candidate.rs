use crate::role::state::{Common, Role};
use crate::role::{Follower, Leader};
use crate::rpc::client::RaftRpcClient;
use crate::rpc::Endpoint;
use crate::RuftError;
use std::cmp::PartialEq;
use std::collections::HashMap;
use std::fmt::Display;
use std::sync::Arc;
use std::time::Duration;

/// Candidate state: requesting votes to become leader
///
#[derive(Clone)]
pub struct Candidate {
    pub my_id: u8,
    pub pre_vote_term: u64,
    pub common: Arc<Common>,
}

impl Role for Candidate {}

impl Display for Candidate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Candidate[pre_vote_term={}]", self.pre_vote_term)
    }
}

pub enum VoteResult {
    Won(
        HashMap<u8, u64>, // id -> committed_index
    ),
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
    async fn pre_vote(&mut self) -> crate::Result<bool> {
        let total_nodes = self.common.voting_clients.len() + 1; // +1 for self
        let majority = (total_nodes / 2) + 1;
        let mut need_grant = majority - 1;
        let mut refuse_grant = majority;

        let (last_log_term, last_log_id) = {
            let guard = self.common.meta.lock().await;
            (guard.last_log_term(), guard.last_log_id())
        };
        let pre_vote_term = self.pre_vote_term + 1;
        let id = self.my_id;

        let rpc_timeout = Duration::from_millis(self.common.config.rpc_timeout_millis);
        let futures: Vec<_> = self
            .common
            .voting_clients
            .iter()
            .map(|entry| {
                let mut client = entry.value().clone();
                async move {
                    tokio::time::timeout(rpc_timeout, client.pre_vote(pre_vote_term, id, last_log_id, last_log_term))
                        .await
                        .map_err(|_| RuftError::Network(format!("Request pre-vote timeout after {:?}", rpc_timeout)))
                        .flatten()
                }
            })
            .collect();

        // fixme: return as soon as possible
        let results = tokio::time::timeout(rpc_timeout, futures::future::join_all(futures))
            .await
            .map_err(|_| RuftError::Network(format!("Wait all pre-vote timeout after {:?}", rpc_timeout)))?;

        for res in results.into_iter().filter_map(Result::ok) {
            if res.vote_granted {
                need_grant = need_grant - 1;
                if need_grant == 0 {
                    return Ok(true);
                }
            } else {
                refuse_grant = refuse_grant - 1;
                if refuse_grant == 0 {
                    return Ok(false);
                }
            }
        }

        Ok(false)
    }
    async fn request_vote(&mut self) -> crate::Result<VoteResult> {
        let total_nodes = self.common.voting_clients.len() + 1; // +1 for self
        let majority = (total_nodes / 2) + 1;
        let mut granted = 1; // Already voted for self
        let mut refused = 0;

        let (request_vote_term, last_log_term, last_log_id) = {
            let mut guard = self.common.meta.lock().await;
            let request_vote_term = guard.next_term()?;
            (request_vote_term, guard.last_log_term(), guard.last_log_id())
        };
        let id = self.my_id;

        let rpc_timeout = Duration::from_millis(self.common.config.rpc_timeout_millis);
        let futures: Vec<_> = self
            .common
            .voting_clients
            .iter()
            .map(|entry| {
                let endpoint = entry.key().clone();
                let mut client = entry.value().clone();
                async move {
                    let result = tokio::time::timeout(rpc_timeout, client.request_vote(request_vote_term, id, last_log_id, last_log_term))
                        .await
                        .map_err(|_| RuftError::Network(format!("Request vote timeout after {:?}", rpc_timeout)))
                        .flatten();
                    result.map(|resp| (endpoint.id(), resp))
                }
            })
            .collect();

        let results = tokio::time::timeout(rpc_timeout, futures::future::join_all(futures))
            .await
            .map_err(|_| RuftError::Network(format!("Wait all vote timeout after {:?}", rpc_timeout)))?;

        let mut last_committed_index: HashMap<u8, u64> = HashMap::new();
        for res in results.into_iter().filter_map(Result::ok) {
            let (peer_id, vote_resp) = res;
            if vote_resp.vote_granted {
                granted = granted + 1;
                last_committed_index.insert(peer_id, vote_resp.committed_index);
                if granted >= majority {
                    return Ok(VoteResult::Won(last_committed_index));
                }
            } else {
                refused = refused - 1;
                if refused >= majority {
                    return Ok(VoteResult::Lost);
                }
            }
        }

        Ok(VoteResult::InProgress)
    }

    /// trigger elect leader
    pub async fn do_electing(&mut self) -> crate::Result<VoteResult> {
        let pre_vote_result = self.pre_vote().await?;

        if pre_vote_result { Ok(VoteResult::Lost) } else { self.request_vote().await }
    }

    /// Discovered a leader - step down to Follower
    pub fn step_down(self, leader_term: u64, leader: Endpoint) -> Follower {
        Follower {
            my_id: self.my_id,
            term: leader_term,
            leader,
            common: self.common,
        }
    }

    /// Won election - become Leader
    pub async fn transition_leader(self, committed_index: HashMap<u8, u64>) -> Leader {
        let term = { self.common.meta.lock().await.term() };
        Leader {
            my_id: self.my_id,
            term,
            next_index: HashMap::new(),
            match_index: committed_index,
            common: self.common,
        }
    }
}
