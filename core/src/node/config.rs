use crate::rpc::Endpoint;

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

    #[test]
    fn test_config_builder() {
        let ep1 = Endpoint::new(1, "localhost".into(), 5001);
        let ep2 = Endpoint::new(2, "localhost".into(), 5002);

        let config = Config::builder().add_member(ep1).add_member(ep2).data_dir("/var/lib/raft").heartbeat_interval(1000).build();

        assert_eq!(config.origin_endpoint.len(), 2);
        assert_eq!(config.data_dir, "/var/lib/raft");
        assert_eq!(config.heartbeat_interval_millis, 1000);
    }
}
