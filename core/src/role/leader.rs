use crate::role::Follower;
use crate::role::replication::Replication;
use crate::role::state::{Common, Role};
use crate::rpc::Endpoint;
use crate::rpc::client::{RemoteClient, init_rpc_clients};
use std::collections::HashMap;
use std::fmt::Display;
use std::sync::Arc;

/// Leader state: managing replication to followers
pub struct Leader {
    my_id: u8,
    term: u64,
    preparing: bool,
    vote_replications: HashMap<u8, Replication>,
    non_vote_replications: HashMap<u8, Replication>,
    common: Arc<Common>,
}

impl Leader {
    pub async fn new(my_id: u8, term: u64, match_index: HashMap<u8, u64>, common: Arc<Common>) -> Self {
        let remote_endpoints = {
            let meta = common.meta.lock().await;
            let members = meta.members();
            let my_endpoint = &common.endpoint;
            members.into_iter().filter(|ep| ep != my_endpoint).collect()
        };
        let (voting, non_voting): (Vec<RemoteClient>, Vec<RemoteClient>) = init_rpc_clients(remote_endpoints).await.into_iter().partition(|c| c.is_voter());

        let voting_clients = voting
            .into_iter()
            .map(|client| {
                let id = client.my_id();
                let match_idx = *match_index.get(&id).clone().unwrap_or(&0);
                let logs = common.logs.clone();
                let replication = Replication::new(client, logs, match_idx);
                (id, replication)
            })
            .collect();
        let non_voting_clients = non_voting
            .into_iter()
            .map(|client| {
                let id = client.my_id();
                let logs = common.logs.clone();
                let replication = Replication::new(client, logs, 0);
                (id, replication)
            })
            .collect();
        Self {
            my_id,
            term,
            preparing: false,
            vote_replications: voting_clients,
            non_vote_replications: non_voting_clients,
            common,
        }
    }

    pub fn term(&self) -> u64 {
        self.term
    }
}

impl Role for Leader {
    fn my_id(&self) -> u8 {
        self.my_id
    }

    fn is_voter(&self) -> bool {
        true
    }

    fn common(&self) -> Arc<Common> {
        self.common.clone()
    }
}

impl Display for Leader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Leader[term={}, followers={}]", self.term, self.vote_replications.len())
    }
}

/// Business logic for Leader role
impl Leader {
    pub async fn become_leader(&mut self) {
        self.non_vote_replications.iter().for_each(|(_, replication)| replication.start());

        // todo: probe msg: heartbeat?
        // todo: merge log entry
        self.preparing = true;
    }

    pub async fn send_append_entries(&self) {
        let meta = self.common.meta.lock().await;

        todo!()
    }

    /// Discovered higher term - step down to Follower
    pub fn transition_follower(self, new_term: u64, leader: Endpoint) -> Follower {
        Follower::new(self.my_id, new_term, leader, self.common)
    }
}

/// Heartbeat request (empty AppendEntries)
#[derive(Debug, Clone)]
pub struct HeartbeatRequest {
    pub term: u64,
    pub leader_id: u8,
    pub prev_log_index: u64,
    pub prev_log_term: u64,
    pub leader_commit: u64,
}
