use crate::rpc::client::RemoteClient;
use crate::storage::LogStore;
use std::sync::Arc;

/// confirmed: 该 entry 已被多数派接受
/// committed: 已提交的index
/// match_index: 与leader保持一致的index

pub(super) struct Replication {
    client: RemoteClient,
    logs: Arc<LogStore>,
    committed: u64,
    match_index: u64,
    inflight: Vec<u64>,
}

impl Replication {
    pub fn new(client: RemoteClient, logs: Arc<LogStore>, committed: u64) -> Self {
        Replication {
            client,
            logs,
            committed,
            match_index: committed,
            inflight: Vec::new(),
        }
    }

    pub fn start(&self) {}
}
