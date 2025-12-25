use crate::command::{CmdReq, CmdResp};
use crate::config::Config;
use crate::endpoint::Endpoint;
use crate::meta::Meta;
use crate::role::state::State;
use std::path::PathBuf;
use crate::rpc::server::start_server;

pub struct Node {
    meta: Meta,
    pub state: State,
}

impl Node {
    pub fn spawn(config: Config) -> Self {
        let node = Node {
            meta: Meta::new(PathBuf::new()),
            state: State::Electing,
        };
        start_server(node);
        node
    }

    fn elect(&self) {

    }

    pub fn update_member(endpoints: Vec<Endpoint>) {}

    pub fn emit(&self, command: CmdReq) -> CmdResp {
        match &self.state {
            State::Electing => CmdResp::Failure {
                message: String::from("Electing"),
            },
            State::Leading { term, leader } => leader.append_entry(command),
            State::Following { term, follower } => CmdResp::Failure {
                message: format!("Following, leader[{}]: {}", term, follower.leader),
            },
            State::Learning { term, learner } => CmdResp::Failure {
                message: format!("Learning, leader[{}]: {}", term, learner.leader),
            },
        }
    }
}
