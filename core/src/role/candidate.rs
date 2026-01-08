use crate::command::{CmdReq, CmdResp};
use crate::endpoint::Endpoint;

pub(crate) struct Candidate {
    endpoint: Endpoint,
    followers: Vec<Endpoint>,
}

impl Candidate {
    pub fn new(endpoint: Endpoint, followers: Vec<Endpoint>) -> Self {
        Candidate { endpoint, followers }
    }

    pub fn pre_vote(&self, _command: CmdReq) -> CmdResp {
        CmdResp::NotLeader { leader: None }
    }
}
