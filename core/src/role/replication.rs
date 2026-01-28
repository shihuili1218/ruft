use std::sync::Arc;
use crate::rpc::client::RemoteClient;

pub(super) struct Replication {
    pub client: RemoteClient,
    pub committed: u64,   // 等价于 nextIndex - 1
    pub match_index: u64, // 可选：用于 commit 计算
    pub inflight: Vec<u64>,
}

impl Replication{
    pub fn new(client: RemoteClient, committed: u64) -> Self{
       Replication {
            client,
            committed,
            match_index: committed,
            inflight: Vec::new(),
        }
    }
    
    pub fn start(&self){}
}