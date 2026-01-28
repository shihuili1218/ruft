use crate::role::state::{Common, Role};
use crate::role::{Follower, Leader};
use crate::rpc::client::{init_rpc_clients, RaftRpcClient, RemoteClient};
use crate::rpc::Endpoint;
use crate::RuftError;
use dashmap::DashMap;
use std::collections::HashMap;
use std::fmt::Display;
use std::sync::Arc;
use std::time::Duration;

/// Candidate state: requesting votes to become leader
///
pub struct Candidate {
    my_id: u8,
    pre_vote_term: u64,
    voting_clients: DashMap<u8, RemoteClient>,
    _non_voting_clients: DashMap<u8, RemoteClient>,
    common: Arc<Common>,
}

impl Candidate {
    pub async fn new(my_id: u8, pre_vote_term: u64, common: Arc<Common>) -> Self {
        let (voting_clients, non_voting_clients) = {
            let meta = common.meta.lock().await;
            let members = meta.members();
            let my_endpoint = &common.endpoint;

            let endpoints = members.into_iter().filter(|ep| ep != my_endpoint).collect();
            init_rpc_clients(endpoints).await
        };

        Self {
            my_id,
            pre_vote_term,
            voting_clients,
            _non_voting_clients: non_voting_clients,
            common,
        }
    }

    pub fn pre_vote_term(&self) -> u64 {
        self.pre_vote_term
    }
}
impl Role for Candidate {
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

impl Display for Candidate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Candidate[pre_vote_term={}]", self.pre_vote_term)
    }
}

pub enum VoteResult {
    Won(HashMap<u8, u64>), // id -> committed_index
    Lost,
    InProgress,
}

/// Business logic for Candidate role
impl Candidate {
    async fn send_pre_vote(&mut self) -> crate::Result<VoteResult> {
        let total_nodes = self.voting_clients.len() + 1; // +1 for self
        let majority = (total_nodes / 2) + 1;
        let mut granted = 1;
        let mut refused = 0;

        let (last_log_term, last_log_id) = {
            let guard = self.common.meta.lock().await;
            (guard.last_log_term(), guard.last_log_id())
        };
        let pre_vote_term = self.pre_vote_term + 1;
        let id = self.my_id;

        let rpc_timeout = Duration::from_millis(self.common.config.rpc_timeout_millis);
        let futures: Vec<_> = self
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
                granted = granted + 1;
                if granted >= majority {
                    return Ok(VoteResult::Won(HashMap::default()));
                }
            } else {
                refused = refused + 1;
                if refused >= majority {
                    return Ok(VoteResult::Lost);
                }
            }
        }

        Ok(VoteResult::InProgress)
    }

    async fn send_request_vote(&mut self) -> crate::Result<VoteResult> {
        let total_nodes = self.voting_clients.len() + 1; // +1 for self
        let majority = (total_nodes / 2) + 1;
        let mut granted = 1; // Already voted for self
        let mut refused = 0;

        let (request_vote_term, last_log_term, last_log_id) = {
            let mut guard = self.common.meta.lock().await;
            let request_vote_term = guard.next_term()?;
            (request_vote_term, guard.last_log_term(), guard.last_log_id())
        };
        let my_id = self.my_id;

        let rpc_timeout = Duration::from_millis(self.common.config.rpc_timeout_millis);
        let futures: Vec<_> = self
            .voting_clients
            .iter()
            .map(|entry| {
                let id = entry.key().clone();
                let mut client = entry.value().clone();
                async move {
                    let result = tokio::time::timeout(rpc_timeout, client.request_vote(request_vote_term, my_id, last_log_id, last_log_term))
                        .await
                        .map_err(|_| RuftError::Network(format!("Request vote timeout after {:?}", rpc_timeout)))
                        .flatten();
                    result.map(|resp| (id, resp))
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
        let pre_vote_result = self.send_pre_vote().await?;
        if let VoteResult::Won(_) = pre_vote_result {
            self.send_request_vote().await
        } else {
            Ok(VoteResult::Lost)
        }
    }

    /// Discovered a leader - step down to Follower
    pub fn transition_follower(self, leader_term: u64, leader: Endpoint) -> Follower {
        Follower::new(self.my_id, leader_term, leader, self.common)
    }

    /// Won election - become Leader
    pub async fn transition_leader(self, committed_index: HashMap<u8, u64>) -> Leader {
        let term = { self.common.meta.lock().await.term() };
        Leader::new(self.my_id, term, committed_index, self.common).await
    }
}
