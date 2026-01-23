use crate::node::meta::PersistentMeta;
use crate::rpc::client::RemoteClient;
use crate::rpc::Endpoint;
use crate::Config;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Marker trait for valid Raft node states
pub trait Role: Sized {}

/// Shared data across all roles
#[derive(Clone)]
pub struct Common {
    pub endpoint: Endpoint,
    pub meta: Arc<Mutex<PersistentMeta>>,
    pub config: Config,
    pub voting_clients: Arc<DashMap<Endpoint, RemoteClient>>,
    pub non_voting_clients: Arc<DashMap<Endpoint, RemoteClient>>,
}
