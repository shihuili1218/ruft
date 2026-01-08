use crate::node::meta::PersistentMeta;
use crate::repeat_timer::{RepeatTimer, RepeatTimerHandle};
use crate::role::{Candidate, Follower, Leader, Learner, RaftState};
use crate::rpc::client::{init_remote_client, RemoteClient};
use crate::rpc::command::{CmdReq, CmdResp};
use crate::rpc::server::run_server;
use crate::rpc::Endpoint;
use crate::{Config, Result, RuftError};
use dashmap::DashMap;
use rand::Rng;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::{error, info};

/// Common data shared across all states
struct CommonData {
    endpoint: Endpoint,
    meta: PersistentMeta,
    config: Config,
    remote_clients: DashMap<Endpoint, RemoteClient>,
    timer: Option<RepeatTimerHandle>,
}

/// Type-safe node with specific state
/// Each state (Follower, Candidate, Leader, Learner) has its own data
struct NodeData<S: RaftState> {
    common: CommonData,
    pub state: S,
}

/// Runtime representation of a Raft node
/// Uses enum to allow state transitions while maintaining type safety per state
pub enum RaftNode {
    Follower(NodeData<Follower>),
    Candidate(NodeData<Candidate>),
    Leader(NodeData<Leader>),
    Learner(NodeData<Learner>),
}

impl RaftNode {
    pub fn new(endpoint: Endpoint, config: Config) -> Result<Self> {
        let meta = PersistentMeta::new(&config)?;
        let term = meta.term();

        let common = CommonData {
            endpoint: endpoint.clone(),
            meta,
            config,
            remote_clients: DashMap::new(),
            timer: None,
        };

        // Start as Follower with a dummy leader (will be updated on first heartbeat)
        let dummy_leader = endpoint;
        Ok(RaftNode::Follower(NodeData {
            common,
            state: Follower {
                term,
                leader: dummy_leader,
                voted_for: None,
            },
        }))
    }

    /// Get common data regardless of current state
    fn common(&self) -> &CommonData {
        match self {
            RaftNode::Follower(node) => &node.common,
            RaftNode::Candidate(node) => &node.common,
            RaftNode::Leader(node) => &node.common,
            RaftNode::Learner(node) => &node.common,
        }
    }

    fn common_mut(&mut self) -> &mut CommonData {
        match self {
            RaftNode::Follower(node) => &mut node.common,
            RaftNode::Candidate(node) => &mut node.common,
            RaftNode::Leader(node) => &mut node.common,
            RaftNode::Learner(node) => &mut node.common,
        }
    }

    pub fn current_term(&self) -> u64 {
        match self {
            RaftNode::Follower(node) => node.state.term(),
            RaftNode::Candidate(node) => node.state.term(),
            RaftNode::Leader(node) => node.state.term(),
            RaftNode::Learner(node) => node.state.term(),
        }
    }

    pub fn state_name(&self) -> &'static str {
        match self {
            RaftNode::Follower(_) => "Follower",
            RaftNode::Candidate(_) => "Candidate",
            RaftNode::Leader(_) => "Leader",
            RaftNode::Learner(_) => "Learner",
        }
    }

    async fn init_rpc_clients(&self) -> Result<()> {
        self.common().remote_clients.clear();

        let members = self.common().meta.members();
        let my_endpoint = &self.common().endpoint;

        for endpoint in members {
            if &endpoint == my_endpoint {
                continue;
            }

            match init_remote_client(&endpoint).await {
                Ok(client) => {
                    self.common().remote_clients.insert(endpoint, client);
                }
                Err(e) => {
                    error!("Failed to init remote client for {}: {}", endpoint, e);
                }
            }
        }

        Ok(())
    }

    pub async fn update_members(&mut self, endpoints: Vec<Endpoint>) -> Result<()> {
        self.common_mut().meta.update_members(endpoints)?;
        self.init_rpc_clients().await?;
        Ok(())
    }

    /// Transition from Follower to Candidate (election timeout)
    pub fn start_election(self) -> Result<Self> {
        if let RaftNode::Follower(mut node) = self {
            let new_term = node.common.meta.next_term()?;
            let id = node.common.endpoint.id();
            let votes = 1; // Vote for self

            Ok(RaftNode::Candidate(NodeData {
                common: node.common,
                state: Candidate {
                    term: new_term,
                    votes_received: votes,
                    voted_for: id,
                },
            }))
        } else {
            // Can only start election from Follower state
            Ok(self)
        }
    }

    /// Transition from Candidate to Leader (won election)
    pub fn become_leader(self) -> Result<Self> {
        if let RaftNode::Candidate(node) = self {
            let members = node.common.meta.members();
            let last_log_index = node.common.meta.log_id();

            // Initialize leader state
            let mut next_index = std::collections::HashMap::new();
            let mut match_index = std::collections::HashMap::new();

            for member in members {
                if member != node.common.endpoint {
                    next_index.insert(member.clone(), last_log_index + 1);
                    match_index.insert(member, 0);
                }
            }

            info!("Node {} became leader for term {}", node.common.endpoint.id(), node.state.term);

            Ok(RaftNode::Leader(NodeData {
                common: node.common,
                state: Leader {
                    term: node.state.term,
                    next_index,
                    match_index,
                },
            }))
        } else {
            Ok(self)
        }
    }

    /// Transition from Candidate to Follower (lost election or discovered higher term)
    pub fn step_down(self, new_term: u64, leader: Endpoint) -> Result<Self> {
        match self {
            RaftNode::Candidate(mut node) => {
                if new_term > node.state.term() {
                    node.common.meta.set_term(new_term)?;
                }

                Ok(RaftNode::Follower(NodeData {
                    common: node.common,
                    state: Follower {
                        term: new_term,
                        leader,
                        voted_for: None,
                    },
                }))
            }
            RaftNode::Leader(mut node) => {
                if new_term > node.state.term() {
                    node.common.meta.set_term(new_term)?;
                }

                Ok(RaftNode::Follower(NodeData {
                    common: node.common,
                    state: Follower {
                        term: new_term,
                        leader,
                        voted_for: None,
                    },
                }))
            }
            // Already follower or learner
            other => Ok(other),
        }
    }

    pub async fn emit(&self, _cmd: CmdReq) -> CmdResp {
        // Only leader can process commands
        match self {
            RaftNode::Leader(_) => {
                // TODO: Implement log replication
                CmdResp::Success { data: None }
            }
            RaftNode::Follower(node) => {
                // Redirect to leader
                CmdResp::NotLeader {
                    leader: Some(node.state.leader.clone()),
                }
            }
            _ => CmdResp::NotLeader { leader: None },
        }
    }
}

/// Wrapper to manage Node with proper locking
pub struct Node {
    // Option allows taking ownership temporarily during state transitions
    inner: Mutex<Option<RaftNode>>,
}

impl Node {
    pub fn new(endpoint: Endpoint, config: Config) -> Result<Self> {
        let node = RaftNode::new(endpoint, config)?;
        Ok(Node { inner: Mutex::new(Some(node)) })
    }

    pub async fn start(self: Arc<Self>) -> Result<()> {
        // Initialize RPC clients
        {
            let guard = self.inner.lock().await;
            if let Some(node) = guard.as_ref() {
                node.init_rpc_clients().await?;
            }
        }

        // Start RPC server
        let _server_handle = tokio::spawn(run_server(self.clone()));

        // Start timer for heartbeat/election
        self.start_timer().await;

        Ok(())
    }

    async fn start_timer(self: &Arc<Self>) {
        let node_for_delay = self.clone();
        let node_for_task = self.clone();

        let timer = RepeatTimer::from_fns(
            "raft_timer".to_string(),
            move || {
                let node = node_for_delay.clone();
                Box::pin(async move {
                    let guard = node.inner.lock().await;
                    if let Some(raft_node) = guard.as_ref() {
                        match raft_node {
                            RaftNode::Candidate(_) => Duration::from_millis(rand::thread_rng().gen_range(150..300)),
                            RaftNode::Follower(_) | RaftNode::Learner(_) => {
                                let config = raft_node.common().config.heartbeat_interval_millis;
                                Duration::from_millis(config + 50)
                            }
                            RaftNode::Leader(_) => {
                                let config = raft_node.common().config.heartbeat_interval_millis;
                                Duration::from_millis(config)
                            }
                        }
                    } else {
                        Duration::from_millis(1000)
                    }
                })
            },
            move || {
                let node = node_for_task.clone();
                Box::pin(async move {
                    let mut guard = node.inner.lock().await;

                    // Take ownership of the node for state transitions
                    if let Some(current_node) = guard.take() {
                        let is_follower = matches!(&current_node, RaftNode::Follower(_));

                        if is_follower {
                            // Heartbeat timeout - become candidate
                            info!("Heartbeat timeout, becoming candidate");
                            match current_node.start_election() {
                                Ok(new_node) => {
                                    *guard = Some(new_node);
                                }
                                Err(e) => {
                                    error!("Failed to start election: {}", e);
                                    // Can't restore current_node after move, create new follower
                                    // This is unlikely to happen as start_election rarely fails
                                    *guard = None;
                                }
                            }
                        } else {
                            match &current_node {
                                RaftNode::Candidate(_) => {
                                    // Election timeout - start new election
                                    info!("Election timeout, starting new election");
                                    // TODO: Send RequestVote RPCs
                                    *guard = Some(current_node);
                                }
                                RaftNode::Leader(_) => {
                                    // Send heartbeat
                                    info!("Sending heartbeat");
                                    // TODO: Send AppendEntries RPCs
                                    *guard = Some(current_node);
                                }
                                RaftNode::Learner(_) => {
                                    // Learner does nothing on timeout
                                    *guard = Some(current_node);
                                }
                                _ => {
                                    *guard = Some(current_node);
                                }
                            }
                        }
                    }
                })
            },
        )
        .spawn();

        // Store timer in the node
        // Note: We need to store it somewhere accessible, for now just keep it alive
        std::mem::forget(timer);
    }

    pub async fn update_members(&self, endpoints: Vec<Endpoint>) -> Result<()> {
        let mut guard = self.inner.lock().await;
        if let Some(node) = guard.as_mut() {
            node.update_members(endpoints).await
        } else {
            Err(RuftError::InvalidState("Node is shutting down".into()))
        }
    }

    pub async fn submit(&self, cmd: CmdReq) -> CmdResp {
        let guard = self.inner.lock().await;
        match guard.as_ref() {
            Some(node) => node.emit(cmd).await,
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
