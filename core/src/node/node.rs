use crate::node::meta::PersistentMeta;
use crate::repeat_timer::{RepeatTimer, RepeatTimerHandle};
use crate::role::{Candidate, Follower, Leader, Learner, Role};
use crate::rpc::client::{init_remote_client, RemoteClient};
use crate::rpc::command::{CmdReq, CmdResp};
use crate::rpc::server::RuftServer;
use crate::rpc::Endpoint;
use crate::{Config, Result, RuftError};
use dashmap::DashMap;
use rand::Rng;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::{error, info};

/// Common data shared across all states
struct Data {
    endpoint: Endpoint,
    meta: PersistentMeta,
    config: Config,
    remote_clients: DashMap<Endpoint, RemoteClient>,
    timer: Option<RepeatTimerHandle>,
}

/// Runtime representation of a Raft node
/// Uses enum to allow state transitions while maintaining type safety per state
enum RaftNode {
    Follower(Data, Follower),
    Candidate(Data, Candidate),
    Leader(Data, Leader),
    Learner(Data, Learner),
}

/// Build Role
impl RaftNode {
    /// Transition from Follower to Candidate (election timeout)
    fn make_candidate(self) -> Result<Self> {
        fn make_candidate(mut common: Data) -> Result<RaftNode> {
            let new_term = common.meta.next_term()?;
            let id = common.endpoint.id();

            Ok(RaftNode::Candidate(
                common,
                Candidate {
                    term: new_term,
                    votes_received: 1,
                    voted_for: id,
                },
            ))
        }

        match self {
            RaftNode::Follower(data, _) => make_candidate(data),
            RaftNode::Leader(data, _) => make_candidate(data),
            RaftNode::Candidate(..) | RaftNode::Learner(..) => Ok(self),
        }
    }

    /// Transition from Candidate to Leader (won election)
    fn make_leader(self) -> Result<Self> {
        if let RaftNode::Candidate(data, node) = self {
            let members = data.meta.members();
            let last_log_index = data.meta.last_log_id();

            // Initialize leader state
            let mut next_index = std::collections::HashMap::new();
            let mut match_index = std::collections::HashMap::new();

            for member in members {
                if member != data.endpoint {
                    next_index.insert(member.clone(), last_log_index + 1);
                    match_index.insert(member, 0);
                }
            }

            info!("Node {} became leader for term {}", data.endpoint.id(), node.term);

            Ok(RaftNode::Leader(
                data,
                Leader {
                    term: node.term,
                    next_index,
                    match_index,
                },
            ))
        } else {
            Ok(self)
        }
    }

    /// Transition from Candidate to Follower (lost election or discovered higher term)
    fn make_follower(self, new_term: u64, leader: Endpoint) -> Result<Self> {
        match self {
            RaftNode::Candidate(mut data, node) => {
                if new_term > node.term() {
                    data.meta.set_term(new_term)?;
                }

                Ok(RaftNode::Follower(data, Follower { term: new_term, leader }))
            }
            RaftNode::Leader(mut data, node) => {
                if new_term > node.term() {
                    data.meta.set_term(new_term)?;
                }

                Ok(RaftNode::Follower(data, Follower { term: new_term, leader }))
            }
            // Already follower or learner
            other => Ok(other),
        }
    }
}

/// Getter
impl RaftNode {
    /// Get common data regardless of current state
    fn common(&self) -> &Data {
        match self {
            RaftNode::Follower(data, _) => data,
            RaftNode::Candidate(data, _) => data,
            RaftNode::Leader(data, _) => data,
            RaftNode::Learner(data, _) => data,
        }
    }

    fn common_mut(&mut self) -> &mut Data {
        match self {
            RaftNode::Follower(data, _) => data,
            RaftNode::Candidate(data, _) => data,
            RaftNode::Leader(data, _) => data,
            RaftNode::Learner(data, _) => data,
        }
    }

    pub fn current_term(&self) -> u64 {
        match self {
            RaftNode::Follower(_, node) => node.term(),
            RaftNode::Candidate(_, node) => node.term(),
            RaftNode::Leader(_, node) => node.term(),
            RaftNode::Learner(_, node) => node.term(),
        }
    }

    pub fn state_name(&self) -> &'static str {
        match self {
            RaftNode::Follower(..) => "Follower",
            RaftNode::Candidate(..) => "Candidate",
            RaftNode::Leader(..) => "Leader",
            RaftNode::Learner(..) => "Learner",
        }
    }
}

impl RaftNode {
    pub fn new(endpoint: Endpoint, config: Config) -> Result<Self> {
        let meta = PersistentMeta::new(&config)?;
        let term = meta.term();

        let common = Data {
            endpoint: endpoint.clone(),
            meta,
            config,
            remote_clients: DashMap::new(),
            timer: None,
        };

        // Start as Follower with a dummy leader (will be updated on first heartbeat)
        let dummy_leader = endpoint;
        Ok(RaftNode::Follower(common, Follower { term, leader: dummy_leader }))
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

    async fn init_rpc_server(&self) -> Result<()> {
        // let server = RuftServer::new(self.clone());
        // server.start().await.map_err(|e| RuftError::Unknown(e.to_string()))?;
        Ok(())
    }

    pub async fn update_members(&mut self, endpoints: Vec<Endpoint>) -> Result<()> {
        self.common_mut().meta.update_members(endpoints)?;
        self.init_rpc_clients().await?;
        Ok(())
    }

    pub async fn submit(&self, _cmd: CmdReq) -> CmdResp {
        // Only leader can process commands
        match self {
            RaftNode::Leader(..) => {
                // TODO: Implement log replication
                CmdResp::Success { data: None }
            }
            RaftNode::Follower(_, node) => {
                // Redirect to leader
                CmdResp::NotLeader { leader: Some(node.leader.clone()) }
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
        {
            let server = RuftServer::new(self.clone());
            let _server_handle = tokio::spawn(server.start());
        }

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
                            RaftNode::Candidate(..) => Duration::from_millis(rand::thread_rng().gen_range(150..300)),
                            RaftNode::Follower(..) | RaftNode::Learner(..) => Duration::from_millis(raft_node.common().config.heartbeat_interval_millis + 50),
                            RaftNode::Leader(..) => Duration::from_millis(raft_node.common().config.heartbeat_interval_millis),
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
                        match current_node {
                            RaftNode::Candidate(..) => {
                                info!("Election timeout, starting new election");
                                // TODO: Send RequestVote RPCs

                                *guard = Some(current_node);
                            }
                            RaftNode::Follower(..) => {
                                info!("Heartbeat timeout, becoming candidate");
                                match current_node.make_candidate() {
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
                            }
                            RaftNode::Leader(..) => {
                                // Send heartbeat
                                info!("Sending heartbeat");
                                // TODO: Send AppendEntries RPCs
                                *guard = Some(current_node);
                            }
                            RaftNode::Learner(..) => {
                                // Learner does nothing on timeout
                                *guard = Some(current_node);
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
            Some(node) => node.submit(cmd).await,
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

impl Node {
    /// Handle RequestVote RPC
    /// Returns (vote_granted, current_term)
    pub async fn on_vote(&self, candidate_id: u8, candidate_term: u64, _last_log_index: u64, _last_log_term: u64) -> Result<(bool, u64)> {
        let (result, current_term) = self.on_pre_vote(candidate_id, candidate_term, _last_log_index, _last_log_term).await?;
        if !result {
            return Ok((false, current_term));
        }

        let mut guard = self.inner.lock().await;

        // First check and set voted_for
        {
            let raft_node = guard.as_mut().ok_or(RuftError::InvalidState("Node shutting down".into()))?;
            let common = raft_node.common_mut();
            let voted_for = common.meta.voted_for();
            let can_vote = voted_for.is_none() || voted_for == Some(candidate_id);
            if !can_vote {
                return Ok((false, raft_node.current_term()));
            }
            common.meta.set_voted_for(candidate_term, candidate_id)?;
        }

        // Then take ownership for state transition
        let current_node = guard.take().ok_or(RuftError::InvalidState("Node shutting down".into()))?;
        let leader_endpoint = current_node.common().meta.get_member(candidate_id)?;

        let (new_node, granted) = match current_node {
            RaftNode::Candidate(..) | RaftNode::Leader(..) => {
                let new_node = current_node.make_follower(candidate_term, leader_endpoint)?;
                (new_node, true)
            }
            RaftNode::Follower(..) => {
                (current_node, true)
            }
            RaftNode::Learner(..) => {
                (current_node, false)
            }
        };

        let term = new_node.current_term();
        *guard = Some(new_node);
        Ok((granted, term))
    }

    /// Handle PreVote RPC (for leadership transfer)
    /// Returns (vote_granted, current_term)
    pub async fn on_pre_vote(&self, _candidate_id: u8, candidate_term: u64, last_log_index: u64, last_log_term: u64) -> Result<(bool, u64)> {
        let guard = self.inner.lock().await;
        let raft_node = guard.as_ref().ok_or(RuftError::InvalidState("Node shutting down".into()))?;
        let current_term = raft_node.current_term();
        let current_log_id = raft_node.common().meta.last_log_id();
        let current_log_term = raft_node.common().meta.last_log_term();

        if let RaftNode::Learner(_, _) = raft_node {
            return Ok((false, current_term));
        }

        // PreVote doesn't change state, just check if we would vote
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
    /// Returns (success, match_index, current_term)
    pub async fn on_append_entries(
        &self,
        _leader_id: u64,
        leader_term: u64,
        _prev_log_index: u64,
        _prev_log_term: u64,
        _entries: Vec<()>, // TODO: Define Entry type
        _leader_commit: u64,
    ) -> Result<(bool, u64, u64)> {
        let mut guard = self.inner.lock().await;
        let raft_node = guard.as_mut().ok_or(RuftError::InvalidState("Node shutting down".into()))?;

        let current_term = raft_node.current_term();

        // Reply false if leader's term < current term
        if leader_term < current_term {
            return Ok((false, 0, current_term));
        }

        // If leader's term >= current term, recognize as leader
        if leader_term >= current_term {
            // TODO: Reset election timer
            // TODO: Transition to follower if needed
        }

        // TODO: Implement log replication logic
        Ok((true, 0, current_term))
    }
}
