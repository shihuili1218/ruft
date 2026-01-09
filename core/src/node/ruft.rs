use crate::Config;
use crate::node::node::Node;
use crate::rpc::Endpoint;
use crate::rpc::command::{CmdReq, CmdResp};
use std::sync::Arc;

/// Main entry point for Raft consensus
///
/// Ruft manages a Raft node and provides a clean API for:
/// - Submitting commands
/// - Querying node state
/// - Managing cluster membership
/// - Graceful shutdown
pub struct Ruft {
    inner: Arc<Node>,
}

impl Ruft {
    /// Create a new Raft node
    ///
    /// # Arguments
    /// * `endpoint` - Network endpoint for this node
    /// * `config` - Configuration parameters
    pub fn new(endpoint: Endpoint, config: Config) -> crate::Result<Self> {
        let node = Node::new(endpoint, config)?;
        Ok(Ruft { inner: Arc::new(node) })
    }

    /// Start the Raft node
    ///
    /// This will:
    /// - Initialize RPC connections to peers
    /// - Start the RPC server
    /// - Begin the election timer
    pub async fn start(&self) -> crate::Result<()> {
        self.inner.clone().start().await
    }

    /// Submit a command to the Raft cluster
    ///
    /// If this node is the leader, the command will be replicated.
    /// If not, returns NotLeader with the known leader endpoint.
    pub async fn submit(&self, cmd: CmdReq) -> CmdResp {
        self.inner.submit(cmd).await
    }

    /// Update cluster membership
    ///
    /// This is a joint-consensus operation in standard Raft.
    /// For now, it's a simple replacement.
    pub async fn update_members(&self, endpoints: Vec<Endpoint>) -> crate::Result<()> {
        self.inner.update_members(endpoints).await
    }

    /// Get the current term
    pub async fn current_term(&self) -> u64 {
        self.inner.current_term().await
    }

    /// Get the current state name (Follower/Candidate/Leader/Learner)
    pub async fn state(&self) -> String {
        self.inner.state_name().await
    }

    /// Check if this node is the leader
    pub async fn is_leader(&self) -> bool {
        self.state().await == "Leader"
    }

    // TODO: Add these methods when needed:
    // - pub async fn shutdown(&self) -> Result<()>
    // - pub async fn snapshot(&self) -> Result<()>
    // - pub async fn get_leader(&self) -> Option<Endpoint>
    // - pub async fn metrics(&self) -> Metrics
}

impl Clone for Ruft {
    fn clone(&self) -> Self {
        Ruft { inner: self.inner.clone() }
    }
}
