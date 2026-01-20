use crate::rpc::Endpoint;
use crate::storage::MmapStorage;
use crate::{Config, Result, RuftError};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone)]
struct Meta {
    initialized: bool,
    term: u64,
    voted_for: Option<u8>,
    last_log_id: u64,
    last_log_term: u64,
    committed_index: u64,
    members: Vec<Endpoint>,
}

pub struct PersistentMeta {
    data: Meta,
    storage: MmapStorage,
}

impl PersistentMeta {
    pub fn new(config: &Config) -> Result<Self> {
        let path = format!("{}/meta.bin", config.data_dir);
        let meta_path = PathBuf::from(&path);
        let storage = MmapStorage::open_or_create(meta_path, 4096).map_err(|e| RuftError::Storage(format!("Failed to open meta file {}: {}", path, e)))?;

        // Try to load existing data, or initialize new
        let data = storage.read_serialized::<Meta>().unwrap_or_else(|_| {
            // Initialize new meta
            Meta {
                initialized: true,
                term: 0,
                voted_for: None,
                last_log_id: 0,
                last_log_term: 0,
                committed_index: 0,
                members: config.origin_endpoint.clone(),
            }
        });

        let mut holder = PersistentMeta { data, storage };

        // Persist if newly initialized
        if !holder.data.initialized {
            holder.data.initialized = true;
            holder.persist()?;
        }

        Ok(holder)
    }

    fn persist(&mut self) -> Result<()> {
        self.storage.write_serialized(&self.data).map_err(|e| RuftError::Storage(format!("Failed to persist meta: {}", e)))
    }

    pub fn next_log_id(&mut self) -> Result<u64> {
        self.data.last_log_id += 1;
        self.data.last_log_term = self.data.term;
        self.persist()?;
        Ok(self.data.last_log_id)
    }

    pub fn next_term(&mut self) -> Result<u64> {
        self.data.term += 1;
        self.data.voted_for = None; // Clear vote when entering new term
        self.persist()?;
        Ok(self.data.term)
    }

    pub fn set_term(&mut self, term: u64) -> Result<()> {
        if term > self.data.term {
            self.data.term = term;
            self.data.voted_for = None;
            self.persist()?;
        }
        Ok(())
    }

    pub fn term(&self) -> u64 {
        self.data.term
    }

    pub fn last_log_id(&self) -> u64 {
        self.data.last_log_id
    }

    pub fn last_log_term(&self) -> u64 {
        self.data.last_log_term
    }

    pub fn committed_index(&self) -> u64 {
        self.data.committed_index
    }

    pub fn set_committed_index(&mut self, index: u64) -> Result<()> {
        if index > self.data.committed_index {
            self.data.committed_index = index;
            self.persist()?;
        }
        Ok(())
    }

    pub fn members(&self) -> Vec<Endpoint> {
        self.data.members.clone()
    }

    pub fn update_members(&mut self, members: Vec<Endpoint>) -> Result<()> {
        self.data.members = members;
        self.persist()
    }

    pub fn get_member(&self, id: u8) -> Result<Endpoint> {
        let endpoint = self.data.members.iter().find(|e| e.id() == id).cloned();
        endpoint.ok_or(RuftError::Unknown(format!("Member not found: {}", id)))
    }

    pub fn voted_for(&self) -> Option<u8> {
        self.data.voted_for
    }

    pub fn set_voted_for(&mut self, term: u64, candidate_id: u8) -> Result<()> {
        self.data.term = term;
        self.data.voted_for = Some(candidate_id);
        self.persist()
    }
}
