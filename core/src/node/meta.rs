use std::path::PathBuf;
use crate::endpoint::Endpoint;

pub struct Meta {
    term: usize,
    log_id: usize,
    members: Vec<Endpoint>, 
}

impl Meta {
    pub fn new(path_buf: PathBuf) -> Self {
        Meta { term: 0, log_id: 0, members: Vec::new() }
    }

    pub fn next_log_id(&mut self) -> usize {
        self.log_id += 1;
        self.log_id
    }
}
