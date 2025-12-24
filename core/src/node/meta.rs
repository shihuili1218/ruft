use std::path::PathBuf;

pub struct Meta {
    term: usize,
    log_id: usize,
}

impl Meta {
    pub fn new(path_buf: PathBuf) -> Self {
        Meta { term: 0, log_id: 0 }
    }

    pub fn next_log_id(&mut self) -> usize {
        self.log_id += 1;
        self.log_id
    }
}
