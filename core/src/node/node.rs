use crate::node::meta::PersistentMeta;
use crate::repeat_timer::RepeatTimer;
use crate::role::{Candidate, Common, Follower, Leader, Learner, Role};
use crate::rpc::Endpoint;
use crate::rpc::client::init_remote_client;
use crate::rpc::command::{CmdReq, CmdResp};
use crate::rpc::server::RuftServer;
use crate::{Config, Result, RuftError};
use dashmap::DashMap;
use rand::Rng;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::{error, info};

/// Runtime representation of a Raft node
/// Each variant holds a complete Role with embedded Common
#[derive(Clone)]
enum RaftNode {
    Follower(Follower),
    Candidate(Candidate),
    Leader(Leader),
    Learner(Learner),
}

impl RaftNode {
    fn current_term(&self) -> u64 {
        match self {
            RaftNode::Follower(r) => r.term(),
            RaftNode::Candidate(r) => r.term(),
            RaftNode::Leader(r) => r.term(),
            RaftNode::Learner(r) => r.term(),
        }
    }

    fn state_name(&self) -> &'static str {
        match self {
            RaftNode::Follower(_) => "Follower",
            RaftNode::Candidate(_) => "Candidate",
            RaftNode::Leader(_) => "Leader",
            RaftNode::Learner(_) => "Learner",
        }
    }

    fn common(&self) -> &Arc<Common> {
        match self {
            RaftNode::Follower(r) => &r.common,
            RaftNode::Candidate(r) => &r.common,
            RaftNode::Leader(r) => &r.common,
            RaftNode::Learner(r) => &r.common,
        }
    }
}

/// Node wrapper with locking
pub struct Node {
    inner: Mutex<Option<RaftNode>>,
}

impl Node {
    pub fn new(endpoint: Endpoint, config: Config) -> Result<Self> {
        let meta = PersistentMeta::new(&config)?;
        let term = meta.term();

        let common = Arc::new(Common {
            endpoint: endpoint.clone(),
            meta: Arc::new(Mutex::new(meta)),
            config,
            voting_clients: Arc::new(DashMap::new()),
            non_voting_clients: Arc::new(DashMap::new()),
        });

        // Start as Follower
        let follower = Follower {
            term,
            leader: endpoint, // Dummy leader
            common,
        };

        Ok(Node {
            inner: Mutex::new(Some(RaftNode::Follower(follower))),
        })
    }

    pub async fn start(self: Arc<Self>) -> Result<()> {
        // Initialize RPC clients
        self.init_rpc_clients().await?;

        // Start RPC server
        let server = RuftServer::new(self.clone());
        tokio::spawn(server.start());

        // Start timer
        self.start_timer().await;

        Ok(())
    }

    async fn init_rpc_clients(&self) -> Result<()> {
        let guard = self.inner.lock().await;
        let node = guard.as_ref().ok_or(RuftError::InvalidState("Node shutting down".into()))?;

        let common = node.common();
        common.voting_clients.clear();
        common.non_voting_clients.clear();

        let meta = common.meta.lock().await;
        let members = meta.members();
        let my_endpoint = &common.endpoint;

        for endpoint in members {
            if &endpoint == my_endpoint {
                continue;
            }

            match init_remote_client(&endpoint).await {
                Ok(client) => {
                    if endpoint.is_voting() {
                        common.voting_clients.insert(endpoint, client);
                    } else {
                        common.non_voting_clients.insert(endpoint, client);
                    }
                }
                Err(e) => {
                    error!("Failed to init remote client for {}: {}", endpoint, e);
                }
            }
        }

        Ok(())
    }

    async fn start_timer(self: &Arc<Self>) {
        let node_for_delay = self.clone();
        let node_for_task = self.clone();

        let timer = RepeatTimer::from_fns(
            "raft_timer".to_string(),
            move || {
                let node = node_for_delay.clone();
                Box::pin(async move { node.timer_interval().await })
            },
            move || {
                let node = node_for_task.clone();
                Box::pin(async move {
                    node.on_timer_tick().await;
                })
            },
        )
        .spawn();

        std::mem::forget(timer);
    }

    async fn timer_interval(&self) -> Duration {
        let guard = self.inner.lock().await;
        if let Some(raft_node) = guard.as_ref() {
            let interval = raft_node.common().config.heartbeat_interval_millis;
            match raft_node {
                RaftNode::Candidate(..) => Duration::from_millis(rand::thread_rng().gen_range(150..300)),
                RaftNode::Follower(..) | RaftNode::Learner(..) => Duration::from_millis(interval + 50),
                RaftNode::Leader(..) => Duration::from_millis(interval),
            }
        } else {
            Duration::from_millis(1000)
        }
    }

    async fn on_timer_tick(&self) {
        let mut guard = self.inner.lock().await;
        let current_node = match guard.take() {
            Some(node) => node,
            None => return,
        };

        let new_node = match current_node {
            RaftNode::Candidate(mut candidate) => {
                info!("Election timeout for term {}, restarting election", candidate.term);
                // candidate.request_vote().await;

                RaftNode::Candidate(candidate)
            }
            RaftNode::Follower(follower) => {
                info!("Heartbeat timeout, starting election");
                match follower.clone().start_election().await {
                    Ok(candidate) => {
                        info!("Became candidate for term {}", candidate.term);
                        // TODO: Send RequestVote RPCs
                        RaftNode::Candidate(candidate)
                    }
                    Err(e) => {
                        error!("Failed to start election: {}", e);
                        RaftNode::Follower(follower)
                    }
                }
            }
            RaftNode::Leader(leader) => {
                info!("Sending heartbeat for term {}", leader.term);
                // TODO: Send AppendEntries RPCs
                RaftNode::Leader(leader)
            }
            RaftNode::Learner(learner) => RaftNode::Learner(learner),
        };

        *guard = Some(new_node);
    }

    pub async fn submit(&self, _cmd: CmdReq) -> CmdResp {
        let guard = self.inner.lock().await;
        match guard.as_ref() {
            Some(RaftNode::Leader(..)) => {
                // TODO: Implement log replication
                CmdResp::Success { data: None }
            }
            Some(RaftNode::Follower(follower)) => CmdResp::NotLeader {
                leader: Some(follower.leader.clone()),
            },
            Some(_) => CmdResp::NotLeader { leader: None },
            None => CmdResp::Rejected {
                code: crate::rpc::command::ErrorCode::Internal,
                message: "Node is shutting down".into(),
            },
        }
    }

    pub async fn current_term(&self) -> u64 {
        let guard = self.inner.lock().await;
        guard.as_ref().map(|n| n.current_term()).unwrap_or(0)
    }

    pub async fn state_name(&self) -> String {
        let guard = self.inner.lock().await;
        guard.as_ref().map(|n| n.state_name().to_string()).unwrap_or_else(|| "Shutdown".to_string())
    }
}

/// RPC handlers
impl Node {
    /// Handle RequestVote RPC
    pub async fn on_vote(&self, candidate_id: u8, candidate_term: u64, last_log_index: u64, last_log_term: u64) -> Result<(bool, u64)> {
        // Pre-vote check
        let (can_vote, current_term) = self.on_pre_vote(candidate_id, candidate_term, last_log_index, last_log_term).await?;
        if !can_vote {
            return Ok((false, current_term));
        }

        let mut guard = self.inner.lock().await;
        let current_node = guard.take().ok_or(RuftError::InvalidState("Node shutting down".into()))?;

        // Learner never votes
        if matches!(current_node, RaftNode::Learner(..)) {
            let term = current_node.current_term();
            *guard = Some(current_node);
            return Ok((false, term));
        }

        let common = current_node.common().clone();

        // Check and set voted_for
        {
            let mut meta = common.meta.lock().await;
            let voted_for = meta.voted_for();
            let can_vote = voted_for.is_none() || voted_for == Some(candidate_id);
            if !can_vote {
                let term = current_node.current_term();
                *guard = Some(current_node);
                return Ok((false, term));
            }
            meta.set_voted_for(candidate_term, candidate_id)?;
        }

        let leader_endpoint = {
            let meta = common.meta.lock().await;
            meta.get_member(candidate_id)?
        };

        // Transition to Follower if necessary
        let new_node = match current_node {
            RaftNode::Candidate(candidate) => {
                let follower = candidate.step_down(candidate_term, leader_endpoint);
                RaftNode::Follower(follower)
            }
            RaftNode::Leader(leader) => {
                let follower = leader.step_down(candidate_term, leader_endpoint);
                RaftNode::Follower(follower)
            }
            RaftNode::Follower(follower) => RaftNode::Follower(follower),
            RaftNode::Learner(_) => unreachable!("Learner already handled"),
        };

        let term = new_node.current_term();
        *guard = Some(new_node);
        Ok((true, term))
    }

    /// Handle PreVote RPC
    pub async fn on_pre_vote(&self, _candidate_id: u8, candidate_term: u64, last_log_index: u64, last_log_term: u64) -> Result<(bool, u64)> {
        let guard = self.inner.lock().await;
        let node = guard.as_ref().ok_or(RuftError::InvalidState("Node shutting down".into()))?;

        if matches!(node, RaftNode::Learner(..)) {
            return Ok((false, node.current_term()));
        }

        let current_term = node.current_term();
        let meta = node.common().meta.lock().await;
        let current_log_id = meta.last_log_id();
        let current_log_term = meta.last_log_term();

        if candidate_term < current_term {
            return Ok((false, current_term));
        }

        if last_log_term < current_log_term {
            return Ok((false, current_term));
        }

        if last_log_term == current_log_term && last_log_index < current_log_id {
            return Ok((false, current_term));
        }

        Ok((true, current_term))
    }

    /// Handle AppendEntries RPC
    pub async fn on_append_entries(&self, leader_id: u8, leader_term: u64, prev_log_index: u64, prev_log_term: u64, entries: Vec<()>, leader_commit: u64) -> Result<(bool, u64, u64)> {
        let mut guard = self.inner.lock().await;
        let current_node = guard.take().ok_or(RuftError::InvalidState("Node shutting down".into()))?;

        let current_term = current_node.current_term();

        // Reject old term
        if leader_term < current_term {
            *guard = Some(current_node);
            return Ok((false, 0, current_term));
        }

        let leader_endpoint = {
            let common = current_node.common();
            let meta = common.meta.lock().await;
            meta.get_member(leader_id)?
        };

        // Process based on current state
        let (new_node, success, match_idx) = match current_node {
            RaftNode::Follower(mut follower) => {
                let (success, idx) = follower
                    .handle_append_entries(leader_term, leader_endpoint.clone(), prev_log_index, prev_log_term, entries, leader_commit)
                    .await?;
                (RaftNode::Follower(follower), success, idx)
            }
            RaftNode::Candidate(candidate) => {
                // Discovered leader, step down
                let mut follower = candidate.step_down(leader_term, leader_endpoint.clone());
                let (success, idx) = follower
                    .handle_append_entries(leader_term, leader_endpoint.clone(), prev_log_index, prev_log_term, entries, leader_commit)
                    .await?;
                (RaftNode::Follower(follower), success, idx)
            }
            RaftNode::Leader(leader) => {
                if leader_term > leader.term {
                    // Discovered higher term, step down
                    let follower = leader.step_down(leader_term, leader_endpoint);
                    (RaftNode::Follower(follower), false, 0)
                } else {
                    // Same term, impossible (two leaders)
                    (RaftNode::Leader(leader), false, 0)
                }
            }
            RaftNode::Learner(learner) => {
                // TODO: Learner should also handle log replication
                (RaftNode::Learner(learner), true, 0)
            }
        };

        let term = new_node.current_term();
        *guard = Some(new_node);
        Ok((success, match_idx, term))
    }

    pub async fn on_pull_snapshot(&self) -> Result<()> {
        Ok(())
    }
}
