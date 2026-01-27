use crate::node::meta::PersistentMeta;
use crate::repeat_timer::RepeatTimer;
use crate::role::{Candidate, Common, Follower, Leader, Learner, Role, VoteResult};
use crate::rpc::client::init_remote_client;
use crate::rpc::command::{CmdReq, CmdResp};
use crate::rpc::server::RuftServer;
use crate::rpc::{AppendEntriesRequest, Endpoint};
use crate::storage::LogStore;
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
    fn my_id(&self) -> u8 {
        match self {
            RaftNode::Follower(r) => r.my_id(),
            RaftNode::Candidate(r) => r.my_id(),
            RaftNode::Leader(r) => r.my_id(),
            RaftNode::Learner(r) => r.my_id(),
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

    fn common(&self) -> Arc<Common> {
        match self {
            RaftNode::Follower(r) => r.common(),
            RaftNode::Candidate(r) => r.common(),
            RaftNode::Leader(r) => r.common(),
            RaftNode::Learner(r) => r.common(),
        }
    }
}

/// Node wrapper with locking
pub struct Node {
    inner: Mutex<Option<RaftNode>>,
}

impl Node {
    pub fn new(my: Endpoint, config: Config) -> Result<Self> {
        let meta = PersistentMeta::new(my.id(), &config)?;
        let term = meta.term();

        let log_store = LogStore::open(config.data_dir.clone())?;

        let common = Arc::new(Common {
            endpoint: my.clone(),
            meta: Arc::new(Mutex::new(meta)),
            logs: Arc::new(log_store),
            config,
            voting_clients: Arc::new(DashMap::new()),
            non_voting_clients: Arc::new(DashMap::new()),
        });

        // Start as Follower with Dummy leader
        let follower = Follower::new(my.id(), term, my, common);

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
                info!("Election timeout for term {}, restarting election", candidate.pre_vote_term());
                let vote_result = candidate.do_electing().await.unwrap_or_else(|err| {
                    error!("request vote failed: {}", err);
                    VoteResult::Lost
                });
                if let VoteResult::Won(committed_index) = vote_result {
                    let leader = candidate.transition_leader(committed_index).await;
                    leader.become_leader().await;
                    RaftNode::Leader(leader)
                } else {
                    RaftNode::Candidate(candidate)
                }
            }
            RaftNode::Follower(follower) => {
                info!("Heartbeat timeout, starting election");
                let candidate = follower.transition_candidate().await;
                RaftNode::Candidate(candidate)
            }
            RaftNode::Leader(leader) => {
                info!("Sending heartbeat for term {}", leader.term());
                leader.heartbeat().await;
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
            Some(RaftNode::Follower(follower)) => CmdResp::NotLeader { leader: Some(follower.leader()) },
            Some(_) => CmdResp::NotLeader { leader: None },
            None => CmdResp::Rejected {
                code: crate::rpc::command::ErrorCode::Internal,
                message: "Node is shutting down".into(),
            },
        }
    }

    pub async fn state_name(&self) -> String {
        let guard = self.inner.lock().await;
        guard.as_ref().map(|n| n.state_name().to_string()).unwrap_or_else(|| "Shutdown".to_string())
    }
}

/// RPC handlers
impl Node {
    /// Handle PreVote RPC
    pub async fn on_pre_vote(&self, _candidate_id: u8, candidate_term: u64, last_log_index: u64, last_log_term: u64) -> Result<(bool, u64)> {
        let guard = self.inner.lock().await;
        let node = guard.as_ref().ok_or(RuftError::InvalidState("Node shutting down".into()))?;

        match node {
            RaftNode::Follower(follower) => follower.handle_pre_vote(_candidate_id, candidate_term, last_log_index, last_log_term).await,
            RaftNode::Candidate(candidate) => candidate.handle_pre_vote(_candidate_id, candidate_term, last_log_index, last_log_term).await,
            RaftNode::Leader(leader) => leader.handle_pre_vote(_candidate_id, candidate_term, last_log_index, last_log_term).await,
            RaftNode::Learner(_) => Err(RuftError::InvalidState("I am Learner".into())),
        }
    }

    /// Handle RequestVote RPC
    pub async fn on_vote(&self, candidate_id: u8, candidate_term: u64, last_log_index: u64, last_log_term: u64) -> Result<(bool, u64)> {
        let mut guard = self.inner.lock().await;

        // Phase 1: Check and gather decision
        let (result, committed_index, should_step_down, leader_endpoint) = match guard.as_ref().ok_or(RuftError::InvalidState("Node shutting down".into()))? {
            RaftNode::Follower(follower) => {
                let (result, committed_index) = follower.handle_vote(candidate_id, candidate_term, last_log_index, last_log_term).await?;
                (result, committed_index, false, None)
            }
            RaftNode::Candidate(candidate) => {
                let (result, committed_index) = candidate.handle_vote(candidate_id, candidate_term, last_log_index, last_log_term).await?;
                let leader_endpoint = if result {
                    let common = candidate.common();
                    let meta = common.meta.lock().await;
                    Some(meta.member(candidate_id)?)
                } else {
                    None
                };
                (result, committed_index, result, leader_endpoint)
            }
            RaftNode::Leader(leader) => {
                let (result, committed_index) = leader.handle_vote(candidate_id, candidate_term, last_log_index, last_log_term).await?;
                let leader_endpoint = if result {
                    let common = leader.common();
                    let meta = common.meta.lock().await;
                    Some(meta.member(candidate_id)?)
                } else {
                    None
                };
                (result, committed_index, result, leader_endpoint)
            }
            RaftNode::Learner(_) => return Err(RuftError::InvalidState("I am Learner".into())),
        };

        // Phase 2: State transformation
        let current_node = guard.take().ok_or(RuftError::InvalidState("Node shutting down".into()))?;
        let new_node = match current_node {
            RaftNode::Follower(follower) => RaftNode::Follower(follower),
            RaftNode::Candidate(candidate) => {
                if should_step_down {
                    RaftNode::Follower(candidate.step_down(candidate_term, leader_endpoint.unwrap()))
                } else {
                    RaftNode::Candidate(candidate)
                }
            }
            RaftNode::Leader(leader) => {
                if should_step_down {
                    RaftNode::Follower(leader.step_down(candidate_term, leader_endpoint.unwrap()))
                } else {
                    RaftNode::Leader(leader)
                }
            }
            RaftNode::Learner(_) => unreachable!(),
        };

        *guard = Some(new_node);
        Ok((result, committed_index))
    }

    async fn step_down_if(&self, leader_id: u8, leader_term: u64) -> Result<u64> {
        let mut guard = self.inner.lock().await;
        let current_node = guard.as_ref().ok_or(RuftError::InvalidState("Node shutting down".into()))?;

        // Phase 1: Check step down logic
        let (should_step_down) = match current_node {
            // Follower doesn't need to step down, just validate
            RaftNode::Follower(_) => false,
            RaftNode::Candidate(candidate) => candidate.check_step_down(leader_id, leader_term).await?,
            RaftNode::Leader(leader) => leader.check_step_down(leader_id, leader_term).await?,
            RaftNode::Learner(_) => return Err(RuftError::InvalidState("I am Learner".into())),
        };

        let (term, leader_endpoint) = {
            let common = current_node.common();
            let meta = common.meta.lock().await;
            let term = meta.term();
            let leader_endpoint = meta.member(leader_id)?;
            (term, leader_endpoint)
        };

        // Phase 2: State transformation
        let current_node = guard.take().ok_or(RuftError::InvalidState("Node shutting down".into()))?;
        let new_node = match current_node {
            RaftNode::Follower(follower) => RaftNode::Follower(follower),
            RaftNode::Candidate(candidate) => {
                if should_step_down {
                    RaftNode::Follower(candidate.step_down(leader_term, leader_endpoint))
                } else {
                    RaftNode::Candidate(candidate)
                }
            }
            RaftNode::Leader(leader) => {
                if should_step_down {
                    RaftNode::Follower(leader.step_down(leader_term, leader_endpoint))
                } else {
                    RaftNode::Leader(leader)
                }
            }
            RaftNode::Learner(_) => unreachable!(),
        };

        *guard = Some(new_node);
        Ok((term))
    }

    /// Handle AppendEntries RPC
    pub async fn on_append_entries(&self, leader_id: u8, leader_term: u64, prev_log_index: u64, prev_log_term: u64, entries: Vec<()>, leader_commit: u64) -> Result<(bool, u64, u64)> {
        let term = self.step_down_if(leader_id, leader_term).await?;

        let guard = self.inner.lock().await;
        let current_node = guard.as_ref().ok_or(RuftError::InvalidState("Node shutting down".into()))?;

        let result_tuple = if let RaftNode::Follower(mut follower) = current_node {
            follower.handle_append_entries(leader_id, leader_term, prev_log_index, prev_log_term, entries, leader_commit).await
        } else {
            Ok((false, 0, term))
        };

        result_tuple
    }

    pub async fn on_pull_snapshot(&self) -> Result<()> {
        Ok(())
    }
}
