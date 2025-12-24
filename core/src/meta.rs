use crate::endpoint::Endpoint;
use std::path::PathBuf;

pub struct Meta {
    pub term: u64,
    log_id: u64,
    pub members: Vec<Endpoint>,
}

impl Meta {
    pub fn new(path_buf: PathBuf) -> Self {
        Meta {
            term: 0,
            log_id: 0,
            members: Vec::new(),
        }
    }

    pub fn next_log_id(&mut self) -> u64 {
        self.log_id = self.log_id + 1;
        self.log_id
    }
}
