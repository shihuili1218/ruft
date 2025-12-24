use crate::command::{CmdReq, CmdResp};
use crate::endpoint::Endpoint;

pub struct Leader {
    endpoint: Endpoint,
    followers: Vec<Endpoint>,
    learners: Vec<Endpoint>,
}

impl Leader {
    pub fn new(endpoint: Endpoint, followers: Vec<Endpoint>, learners: Vec<Endpoint>) -> Self {
        Leader {
            endpoint,
            followers,
            learners,
        }
    }

    pub fn append_entry(&self, command: CmdReq) -> CmdResp {
        CmdResp::Failure {
            message: String::new(),
        }
    }
}
