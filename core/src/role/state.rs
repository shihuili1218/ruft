use crate::node::meta::PersistentMeta;
use crate::rpc::client::RemoteClient;
use crate::rpc::Endpoint;
use crate::Config;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Marker trait for valid Raft node states
pub trait Role: Sized {
    fn term(&self) -> u64;
    fn state_name() -> &'static str;
}

/// Shared data across all roles
#[derive(Clone)]
pub struct Common {
    pub endpoint: Endpoint,
    pub meta: Arc<Mutex<PersistentMeta>>,
    pub config: Config,
    pub remote_clients: Arc<DashMap<Endpoint, RemoteClient>>,
}
