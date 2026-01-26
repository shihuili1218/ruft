use crate::node::meta::PersistentMeta;
use crate::rpc::client::RemoteClient;
use crate::rpc::Endpoint;
use crate::{Config, RuftError};
use crate::Result;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::storage::LogStore;

/// Marker trait for valid Raft node states
pub trait Role: Sized {
    fn my_id(&self) -> u8;
    fn is_voter(&self) -> bool;
    fn common(&self) -> Arc<Common>;

    async fn handle_pre_vote(&self, _candidate_id: u8, candidate_term: u64, last_log_index: u64, last_log_term: u64) -> Result<(bool, u64)> {
        let common = self.common();
        let meta = common.meta.lock().await;

        let committed_index = meta.committed_index();
        let current_term = meta.term();
        let current_log_id = meta.last_log_id();
        let current_log_term = meta.last_log_term();

        if !self.is_voter() {
            return Ok((false, committed_index));
        }

        if candidate_term < current_term {
            return Ok((false, committed_index));
        }

        if last_log_term < current_log_term {
            return Ok((false, committed_index));
        }

        if last_log_term == current_log_term && last_log_index < current_log_id {
            return Ok((false, committed_index));
        }

        Ok((true, committed_index))
    }

    async fn handle_vote(&self, candidate_id: u8, candidate_term: u64, last_log_index: u64, last_log_term: u64) -> Result<(bool, u64)> {
        // pre-vote check
        let (can_vote, committed_index) = self.handle_pre_vote(candidate_id, candidate_term, last_log_index, last_log_term).await?;
        if !can_vote {
            return Ok((false, committed_index));
        }

        let common = self.common();

        // Check and set voted_for
        {
            let mut meta = common.meta.lock().await;
            let voted_for = meta.voted_for();
            let can_vote = voted_for.is_none() || voted_for == Some(candidate_id);
            if !can_vote {
                return Ok((false, committed_index));
            }
            meta.set_voted_for(candidate_term, candidate_id)?;
        }

        Ok((true, committed_index))

    }

    async fn check_step_down(&self, remote_id: u8, remote_term: u8) -> Result<bool> {

        todo!()
    }

}

/// Shared data across all roles
#[derive(Clone)]
pub struct Common {
    pub endpoint: Endpoint,
    pub meta: Arc<Mutex<PersistentMeta>>,
    pub logs: Arc<LogStore>,
    pub config: Config,
    pub voting_clients: Arc<DashMap<Endpoint, RemoteClient>>,
    pub non_voting_clients: Arc<DashMap<Endpoint, RemoteClient>>,
}
