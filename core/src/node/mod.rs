mod meta;
pub(crate) mod node;

use crate::command::{CmdReq, CmdResp};
use crate::endpoint::Endpoint;
use crate::node::node::Node;
use crate::Result;
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
    /// * `id` - Unique identifier for this node
    /// * `endpoint` - Network endpoint for this node
    /// * `config` - Configuration parameters
    pub fn new(id: u64, endpoint: Endpoint, config: Config) -> Result<Self> {
        let node = Node::new(id, endpoint, config)?;
        Ok(Ruft {
            inner: Arc::new(node),
        })
    }

    /// Start the Raft node
    ///
    /// This will:
    /// - Initialize RPC connections to peers
    /// - Start the RPC server
    /// - Begin the election timer
    pub async fn start(&self) -> Result<()> {
        self.inner.clone().start().await
    }

    /// Submit a command to the Raft cluster
    ///
    /// If this node is the leader, the command will be replicated.
    /// If not, returns NotLeader with the known leader endpoint.
    pub async fn submit(&self, cmd: CmdReq) -> CmdResp {
        self.inner.emit(cmd).await
    }

    /// Update cluster membership
    ///
    /// This is a joint-consensus operation in standard Raft.
    /// For now, it's a simple replacement.
    pub async fn update_members(&self, endpoints: Vec<Endpoint>) -> Result<()> {
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
        Ruft {
            inner: self.inner.clone(),
        }
    }
}

/// Configuration for a Raft node
#[derive(Clone, Debug)]
pub struct Config {
    pub origin_endpoint: Vec<Endpoint>,
    pub data_dir: String,
    pub heartbeat_interval_millis: u64,
}

impl Config {
    /// Create a new config builder
    pub fn builder() -> ConfigBuilder {
        ConfigBuilder::default()
    }

    /// Deprecated: use Config::builder() instead
    #[deprecated(since = "0.2.0", note = "Use Config::builder() instead")]
    pub fn new(endpoints: Vec<Endpoint>) -> Self {
        Config {
            origin_endpoint: endpoints,
            ..Default::default()
        }
    }

    /// Deprecated: use Config::builder() instead
    #[deprecated(since = "0.2.0", note = "Use Config::builder().data_dir() instead")]
    pub fn with_data_dir(mut self, dir: impl Into<String>) -> Self {
        self.data_dir = dir.into();
        self
    }

    /// Deprecated: use Config::builder() instead
    #[deprecated(since = "0.2.0", note = "Use Config::builder().heartbeat_interval() instead")]
    pub fn with_heartbeat_interval(mut self, millis: u64) -> Self {
        self.heartbeat_interval_millis = millis;
        self
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            origin_endpoint: vec![],
            data_dir: "/tmp/ruft".into(),
            heartbeat_interval_millis: 3000,
        }
    }
}

/// Builder for Config with full chain-able API
#[derive(Default)]
pub struct ConfigBuilder {
    endpoints: Vec<Endpoint>,
    data_dir: Option<String>,
    heartbeat_interval: Option<u64>,
}

impl ConfigBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the initial cluster members
    pub fn members(mut self, endpoints: Vec<Endpoint>) -> Self {
        self.endpoints = endpoints;
        self
    }

    /// Add a single member to the cluster
    pub fn add_member(mut self, endpoint: Endpoint) -> Self {
        self.endpoints.push(endpoint);
        self
    }

    /// Set the data directory for persistent storage
    pub fn data_dir(mut self, dir: impl Into<String>) -> Self {
        self.data_dir = Some(dir.into());
        self
    }

    /// Set the heartbeat interval in milliseconds
    pub fn heartbeat_interval(mut self, millis: u64) -> Self {
        self.heartbeat_interval = Some(millis);
        self
    }

    /// Build the Config
    pub fn build(self) -> Config {
        Config {
            origin_endpoint: self.endpoints,
            data_dir: self.data_dir.unwrap_or_else(|| "/tmp/ruft".into()),
            heartbeat_interval_millis: self.heartbeat_interval.unwrap_or(3000),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::endpoint::Address;

    #[test]
    fn test_config_builder() {
        let ep1 = Endpoint::new(1, Address::new("localhost".into(), 5001));
        let ep2 = Endpoint::new(2, Address::new("localhost".into(), 5002));

        let config = Config::builder()
            .add_member(ep1)
            .add_member(ep2)
            .data_dir("/var/lib/raft")
            .heartbeat_interval(1000)
            .build();

        assert_eq!(config.origin_endpoint.len(), 2);
        assert_eq!(config.data_dir, "/var/lib/raft");
        assert_eq!(config.heartbeat_interval_millis, 1000);
    }
}
