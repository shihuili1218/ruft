use crate::endpoint::Endpoint;
use std::path::PathBuf;

#[repr(C)]
pub struct Meta {
    pub term: u64,
    pub log_id: u64,
    pub committed_index: u64,
    pub members: Vec<Endpoint>,
}

impl Meta {



    pub fn read_or_create(path: PathBuf) -> Self {
        Meta {
            term: 0,
            log_id: 0,
            committed_index: 0,
            members: Vec::new(),
        }
    }

    pub fn next_log_id(&mut self) -> u64 {
        self.log_id += 1;
        self.log_id
    }
}
